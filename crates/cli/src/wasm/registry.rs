use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{
  collections::HashMap,
  fs,
  io::{Read, Write},
  path::{Path, PathBuf},
  time::Instant,
};
use url::Url;
use wasmtime::{Engine, component::Component};

pub struct ComponentRegistry {
  engine: Engine,
  components: HashMap<String, Component>,
  cache_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct ComponentMetadata {
  url: Url,
  hash: String,
}

fn read_metadata(path: &Path) -> Result<Option<ComponentMetadata>> {
  if !path.is_file() {
    return Ok(None);
  }
  let content = match fs::read_to_string(path) {
    Ok(content) => content,
    Err(err) => {
      log::warn!("Failed to read wasm metadata {path:?}: {err}");
      return Ok(None);
    }
  };
  match toml::from_str::<ComponentMetadata>(&content) {
    Ok(metadata) => Ok(Some(metadata)),
    Err(err) => {
      log::warn!("Failed to parse wasm metadata {path:?}: {err}");
      Ok(None)
    }
  }
}

fn write_metadata(path: &Path, metadata: &ComponentMetadata) -> Result<()> {
  let content = toml::to_string(metadata).context("Failed to serialize wasm metadata")?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).context("Failed to ensure wasm metadata dir")?;
  }
  fs::write(path, content).context("Failed to write wasm metadata")
}

fn download_to_path(url: &Url, path: &Path) -> Result<String> {
  let response = ureq::get(url.as_str())
    .call()
    .context("Failed to download wasm component")?;
  let mut reader = response.into_reader();

  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).context("Failed to ensure wasm download dir")?;
  }

  let tmp_path = path.with_extension("tmp");
  let mut file = fs::File::create(&tmp_path).context("Failed to create wasm download file")?;
  let mut hasher = sha2::Sha256::new();
  let mut buffer = [0_u8; 8192];
  loop {
    let read = reader
      .read(&mut buffer)
      .context("Failed to read wasm download")?;
    if read == 0 {
      break;
    }
    file
      .write_all(&buffer[..read])
      .context("Failed to write wasm download")?;
    hasher.update(&buffer[..read]);
  }
  file.flush().context("Failed to flush wasm download")?;
  fs::rename(&tmp_path, path).context("Failed to persist wasm download")?;

  Ok(format!("{:x}", hasher.finalize()))
}

fn hash_file(path: &Path) -> Result<String> {
  let mut file = fs::File::open(path).context("Failed to open wasm component file")?;
  let mut hasher = sha2::Sha256::new();
  let mut buffer = [0_u8; 8192];
  loop {
    let read = file
      .read(&mut buffer)
      .context("Failed to read wasm component file")?;
    if read == 0 {
      break;
    }
    hasher.update(&buffer[..read]);
  }
  Ok(format!("{:x}", hasher.finalize()))
}

impl ComponentRegistry {
  pub fn new(engine: Engine, cache_dir: PathBuf) -> Self {
    Self {
      engine,
      components: HashMap::new(),
      cache_dir,
    }
  }

  pub fn has_component(&self, name: &str) -> bool {
    self.components.contains_key(name)
  }

  pub fn get_component(&self, name: &str) -> Option<&Component> {
    self.components.get(name)
  }

  fn compile_component(&mut self, name: &str, path: &Path, hash: &str) -> Result<Component> {
    let cache_path = self
      .cache_dir
      .join("wasm")
      .join(name)
      .join("compiled")
      .join(format!("{hash}.cwasm"));

    if std::fs::exists(&cache_path)? {
      return unsafe { Component::deserialize_file(&self.engine, cache_path) };
    }

    let component = wasmtime::component::Component::from_file(&self.engine, path)
      .context("Failed to load wasm formatter from file")?;

    let serialized = component
      .serialize()
      .context("Faield to serialize wasm component for cache")?;
    if let Some(parent) = cache_path.parent() {
      fs::create_dir_all(parent).context("Failed to ensure cache dir")?;
    }
    fs::write(&cache_path, serialized).context("Failed to write wasm component cache")?;

    Ok(component)
  }

  pub fn load_component(&mut self, name: &str, url: &Url) -> Result<()> {
    let start = Instant::now();

    let (path, hash) = self.resolve_component_source(name, url)?;
    let component = self.compile_component(name, &path, &hash)?;
    self.components.insert(name.into(), component);

    log::debug!(
      "Component [{name}] loaded in: {:?}",
      Instant::now().duration_since(start)
    );

    Ok(())
  }

  fn resolve_component_source(&self, name: &str, url: &Url) -> Result<(PathBuf, String)> {
    match url.scheme() {
      "file" => self.resolve_file_component(url),
      "http" | "https" => self.resolve_remote_component(name, url),
      scheme => anyhow::bail!("Unsupported wasm component scheme: {scheme}"),
    }
  }

  fn resolve_file_component(&self, url: &Url) -> Result<(PathBuf, String)> {
    let path = url
      .to_file_path()
      .map_err(|_| anyhow::anyhow!("Invalid file url: {url}"))?;
    let hash = hash_file(&path).context("Failed to hash wasm component file")?;
    Ok((path, hash))
  }

  fn resolve_remote_component(&self, name: &str, url: &Url) -> Result<(PathBuf, String)> {
    let component_dir = self.cache_dir.join("wasm").join(name);
    fs::create_dir_all(&component_dir).context("Failed to ensure wasm cache dir")?;

    let metadata_path = component_dir.join("metadata.toml");
    let download_path = component_dir.join("component.wasm");

    if let Some(metadata) = read_metadata(&metadata_path)? {
      if metadata.url == *url && download_path.is_file() {
        return Ok((download_path, metadata.hash));
      }
    }

    let hash = download_to_path(url, &download_path)?;
    let metadata = ComponentMetadata {
      url: url.clone(),
      hash: hash.clone(),
    };
    write_metadata(&metadata_path, &metadata)?;

    Ok((download_path, hash))
  }
}
