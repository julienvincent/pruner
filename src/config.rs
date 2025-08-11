use anyhow::Result;
use std::{collections::HashMap, path::{Path, PathBuf}};
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

#[derive(serde::Deserialize, Debug)]
pub struct FormatterSpec {
  pub cmd: String,
  pub args: Vec<String>,
  pub stdin: Option<bool>,
  pub fail_on_stderr: Option<bool>,
}

pub type FormatterSpecs = HashMap<String, FormatterSpec>;
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
}

impl PrunerConfig {
  pub fn from_file(path: &Path) -> Result<Self> {
    let content = std::fs::read_to_string(path)?;
    let config: PrunerConfig = toml::from_str(&content)?;
    Ok(config)
  }
}
