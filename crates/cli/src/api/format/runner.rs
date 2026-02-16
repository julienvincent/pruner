use anyhow::{Context, Result};
use std::{
  fs,
  io::Write,
  path::{Path, PathBuf},
  process::{Command, Stdio},
  time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::config::FormatterSpec;

#[derive(Debug)]
pub struct FormatOpts<'a> {
  pub printwidth: u32,
  pub language: &'a str,
  pub source_file: Option<&'a Path>,
}

fn unique_temp_file(
  formatter: &FormatterSpec,
  source_file: Option<&Path>,
) -> std::io::Result<PathBuf> {
  let mut path = if formatter.colocate_temp_file.unwrap_or(false) {
    source_file
      .and_then(|file| file.parent())
      .map(Path::to_path_buf)
      .unwrap_or_else(std::env::temp_dir)
  } else {
    std::env::temp_dir()
  };

  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos();

  let root_file_name = source_file
    .and_then(|file| file.file_stem())
    .and_then(|stem| stem.to_str())
    .filter(|name| !name.is_empty())
    .unwrap_or("stdin");

  let file_ext = formatter
    .file_ext
    .as_deref()
    .unwrap_or("")
    .trim_start_matches('.');

  let random = format!("{}-{nanos}", std::process::id());
  let file_name = if file_ext.is_empty() {
    format!(".pruner.{root_file_name}.{random}")
  } else {
    format!(".pruner.{root_file_name}.{random}.{file_ext}")
  };
  path.push(file_name);

  Ok(path)
}

pub fn format(
  formatter: &FormatterSpec,
  source: &[u8],
  opts: &FormatOpts,
) -> Result<Vec<u8>> {
  log::trace!("Calling formatter [{}] with opts {:?}", formatter.cmd, opts);

  let use_stdin = formatter.stdin.unwrap_or(true);
  let use_stout = if !use_stdin {
    formatter.stdout.unwrap_or(false)
  } else {
    true
  };

  let mut temp_file: Option<PathBuf> = None;

  if !use_stdin {
    let path = unique_temp_file(formatter, opts.source_file)
      .context("Failed to create temp file for fomatting")?;
    fs::write(&path, source).context("Failed to write to temp file")?;
    temp_file = Some(path);
  }

  let file_var = temp_file
    .as_ref()
    .map(|path| path.to_string_lossy().to_string())
    .unwrap_or_default();

  let args = formatter.args.iter().map(|arg| {
    arg
      .replace("$textwidth", &format!("{}", opts.printwidth))
      .replace("$language", opts.language)
      .replace("$file", &file_var)
  });

  let mut command = Command::new(&formatter.cmd);
  command
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .stdin(Stdio::piped());

  let start = Instant::now();

  let result = || -> Result<Vec<u8>> {
    let mut proc = command.spawn()?;

    if use_stdin {
      let stdin = proc
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Failed to open stdin"))?;
      stdin.write_all(source)?;
    }

    let output = proc.wait_with_output()?;

    if !output.status.success() {
      anyhow::bail!(
        "Failed to run formatter {}: {}",
        formatter.cmd,
        String::from_utf8_lossy(&output.stderr)
      );
    }

    if formatter.fail_on_stderr.unwrap_or(false) && !output.stderr.is_empty() {
      anyhow::bail!(
        "Failed to run formatter {}: {}",
        formatter.cmd,
        String::from_utf8_lossy(&output.stderr)
      );
    }

    let mut result = output.stdout;

    if !use_stout && let Some(path) = temp_file.as_ref() {
      result = fs::read(path).context("Failed to read temp file after formatting")?;
    }

    Ok(result)
  }();

  log::debug!(
    "Formatted using [{}] in: {:?}",
    formatter.cmd,
    Instant::now().duration_since(start)
  );

  if let Some(ref path) = temp_file
    && let Err(err) = fs::remove_file(path)
  {
    log::warn!("Failed to remove temp file {path:?}: {err}");
  }

  match result {
    Ok(result) => {
      if result.is_empty() {
        Err(anyhow::format_err!(
          "Unexpected empty result received from command: {}",
          formatter.cmd
        ))
      } else {
        Ok(result)
      }
    }
    Err(err) => Err(err),
  }
}

#[cfg(test)]
mod tests {
  use super::unique_temp_file;
  use crate::config::FormatterSpec;
  use std::path::Path;

  #[test]
  fn uses_expected_temp_file_name_shape() {
    let formatter = FormatterSpec {
      cmd: "dummy".into(),
      file_ext: Some(".clj".into()),
      ..Default::default()
    };

    let path = unique_temp_file(&formatter, Some(Path::new("/workspace/src/main.nix"))).unwrap();
    let name = path.file_name().unwrap().to_string_lossy();

    assert!(name.starts_with(".pruner.main."));
    assert!(name.ends_with(".clj"));
    assert!(!name.contains("..clj"));
  }

  #[test]
  fn colocates_temp_file_with_source_when_enabled() {
    let formatter = FormatterSpec {
      cmd: "dummy".into(),
      colocate_temp_file: Some(true),
      ..Default::default()
    };

    let source_file = Path::new("/workspace/src/main.clj");
    let path = unique_temp_file(&formatter, Some(source_file)).unwrap();

    assert_eq!(path.parent().unwrap(), source_file.parent().unwrap());
  }

  #[test]
  fn falls_back_to_os_temp_dir_when_source_is_missing() {
    let formatter = FormatterSpec {
      cmd: "dummy".into(),
      colocate_temp_file: Some(true),
      ..Default::default()
    };

    let path = unique_temp_file(&formatter, None).unwrap();

    assert_eq!(path.parent().unwrap(), std::env::temp_dir());
  }
}
