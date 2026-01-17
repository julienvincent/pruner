use anyhow::Result;
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  process::Command,
};
use url::Url;

use crate::config::GrammarSpec;

pub struct CloneArgs<'a> {
  pub repo: &'a Url,
  pub target_dir: &'a PathBuf,
  pub rev: Option<&'a str>,
}
pub fn clone(args: CloneArgs) -> Result<()> {
  if args.target_dir.exists() {
    return Ok(());
  }

  log::info!("Cloning {} ...", args.repo);

  let mut clone_args = Vec::from(["clone", "--depth", "1"]);
  if let Some(rev) = args.rev {
    clone_args.push("--revision");
    clone_args.push(rev);
  }

  clone_args.push(args.repo.as_str());
  clone_args.push(args.target_dir.to_str().ok_or(anyhow::format_err!(
    "Could not convert target dir to string"
  ))?);

  let status = Command::new("git").args(clone_args).status()?;
  if !status.success() {
    anyhow::bail!("Failed to clone repo: {status}");
  }
  Ok(())
}

pub fn clone_all_grammars(
  clone_path: &Path,
  grammars: &HashMap<String, GrammarSpec>,
) -> Result<()> {
  for (lang, spec) in grammars {
    clone(CloneArgs {
      repo: spec.url(),
      target_dir: &clone_path.join(lang),
      rev: spec.rev(),
    })?;
  }
  Ok(())
}
