use anyhow::{Context, Result};
use std::{
  collections::HashMap,
  hash::Hash,
  path::{Path, PathBuf},
};
use url::Url;

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum GrammarSpec {
  Url(Url),
  Table { url: Url, rev: Option<String> },
}

impl GrammarSpec {
  pub fn url(&self) -> &Url {
    match self {
      GrammarSpec::Url(url) => url,
      GrammarSpec::Table { url, .. } => url,
    }
  }

  pub fn rev(&self) -> Option<&str> {
    match self {
      GrammarSpec::Url(_) => None,
      GrammarSpec::Table { rev, .. } => match rev {
        Some(rev) => Some(rev),
        None => None,
      },
    }
  }
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
pub struct FormatterSpec {
  pub cmd: String,
  pub args: Vec<String>,
  pub stdin: Option<bool>,
  pub fail_on_stderr: Option<bool>,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum WasmComponentSpec {
  Url(Url),
  Table { url: Url },
}

impl WasmComponentSpec {
  pub fn url(&self) -> &Url {
    match self {
      Self::Url(url) => url,
      Self::Table { url, .. } => url,
    }
  }
}

pub type FormatterSpecs = HashMap<String, FormatterSpec>;
pub type WasmComponentSpecs = HashMap<String, WasmComponentSpec>;
pub type GrammarSpecs = HashMap<String, GrammarSpec>;

pub type LanguageFormatSpec = Vec<String>;
pub type LanguageFormatters = HashMap<String, LanguageFormatSpec>;

#[derive(serde::Deserialize, Debug, Default)]
pub struct PrunerConfig {
  pub query_paths: Option<Vec<PathBuf>>,
  pub grammar_paths: Option<Vec<PathBuf>>,

  pub grammar_download_dir: Option<PathBuf>,
  pub grammar_build_dir: Option<PathBuf>,

  pub grammars: Option<GrammarSpecs>,
  pub languages: Option<LanguageFormatters>,
  pub formatters: Option<FormatterSpecs>,
  pub wasm_formatters: Option<WasmComponentSpecs>,
}

fn absolutize_vec(paths: Vec<PathBuf>, base_dir: &Path) -> Vec<PathBuf> {
  paths
    .into_iter()
    .map(|path| absolutize_path(path, base_dir))
    .collect()
}

fn absolutize_path(path: PathBuf, base_dir: &Path) -> PathBuf {
  if path.is_absolute() {
    path
  } else {
    base_dir.join(path)
  }
}

fn merge_vecs<T: Clone>(base: &Option<Vec<T>>, overlay: &Option<Vec<T>>) -> Option<Vec<T>> {
  match (base, overlay) {
    (None, None) => None,
    (Some(values), None) | (None, Some(values)) => Some(values.clone()),
    (Some(base_values), Some(overlay_values)) => {
      let mut merged = base_values.clone();
      merged.extend(overlay_values.clone());
      Some(merged)
    }
  }
}

fn merge_maps<K: Eq + Hash + Clone, V: Clone>(
  base: &Option<HashMap<K, V>>,
  overlay: &Option<HashMap<K, V>>,
) -> Option<HashMap<K, V>> {
  match (base, overlay) {
    (None, None) => None,
    (Some(values), None) | (None, Some(values)) => Some(values.clone()),
    (Some(base_values), Some(overlay_values)) => {
      let mut merged = base_values.clone();
      merged.extend(overlay_values.clone());
      Some(merged)
    }
  }
}

impl PrunerConfig {
  pub fn from_file(path: &Path) -> Result<Self> {
    let content = std::fs::read_to_string(path)?;
    let config: PrunerConfig = toml::from_str(&content)?;
    Ok(config.absolutize_paths(path.parent()))
  }

  pub fn merge(base: &PrunerConfig, overlay: &PrunerConfig) -> PrunerConfig {
    PrunerConfig {
      query_paths: merge_vecs(&base.query_paths, &overlay.query_paths),
      grammar_paths: merge_vecs(&base.grammar_paths, &overlay.grammar_paths),
      grammar_download_dir: overlay
        .grammar_download_dir
        .clone()
        .or_else(|| base.grammar_download_dir.clone()),
      grammar_build_dir: overlay
        .grammar_build_dir
        .clone()
        .or_else(|| base.grammar_build_dir.clone()),
      grammars: merge_maps(&base.grammars, &overlay.grammars),
      languages: merge_maps(&base.languages, &overlay.languages),
      formatters: merge_maps(&base.formatters, &overlay.formatters),
      wasm_formatters: merge_maps(&base.wasm_formatters, &overlay.wasm_formatters),
    }
  }

  fn absolutize_paths(mut self, base_dir: Option<&Path>) -> Self {
    let Some(base_dir) = base_dir else {
      return self;
    };

    self.query_paths = self
      .query_paths
      .map(|paths| absolutize_vec(paths, base_dir));
    self.grammar_paths = self
      .grammar_paths
      .map(|paths| absolutize_vec(paths, base_dir));
    self.grammar_download_dir = self
      .grammar_download_dir
      .map(|path| absolutize_path(path, base_dir));
    self.grammar_build_dir = self
      .grammar_build_dir
      .map(|path| absolutize_path(path, base_dir));

    self
  }
}

fn find_local_config(start_dir: &Path) -> Option<PathBuf> {
  for ancestor in start_dir.ancestors() {
    let candidate = ancestor.join("pruner.toml");
    if candidate.is_file() {
      return Some(candidate);
    }
  }
  None
}

pub fn load(config_path: Option<PathBuf>) -> Result<PrunerConfig> {
  let cwd = std::env::current_dir()?;

  if let Some(path) = config_path {
    return PrunerConfig::from_file(&cwd.join(path));
  }

  let xdg_dirs = xdg::BaseDirectories::with_prefix("pruner");
  let config_path = xdg_dirs.find_config_file("config.toml");
  let global_config = match config_path.as_deref() {
    Some(config_path) => PrunerConfig::from_file(config_path)
      .with_context(|| format!("Failed to load config {:?}", config_path))?,
    None => PrunerConfig::default(),
  };

  let local_config_path = find_local_config(&cwd);
  let local_config = match local_config_path.as_deref() {
    Some(local_config_path) => PrunerConfig::from_file(local_config_path)
      .with_context(|| format!("Failed to load config {:?}", local_config_path))?,
    None => PrunerConfig::default(),
  };

  Ok(PrunerConfig::merge(&global_config, &local_config))
}
