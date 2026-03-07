use anstream::{AutoStream, ColorChoice};
use anstyle::{AnsiColor, Effects, Style};
use anyhow::Result;
use similar::TextDiff;
use std::io::Write;
use std::sync::{LazyLock, Mutex};

static STDERR_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn style_line(line: &str) -> String {
  let style = if line.starts_with("+++") || line.starts_with("---") {
    Style::new()
      .effects(Effects::BOLD)
      .fg_color(Some(AnsiColor::Yellow.into()))
  } else if line.starts_with("@@") {
    Style::new().fg_color(Some(AnsiColor::Cyan.into()))
  } else if line.starts_with('+') {
    Style::new().fg_color(Some(AnsiColor::Green.into()))
  } else if line.starts_with('-') {
    Style::new().fg_color(Some(AnsiColor::Red.into()))
  } else {
    return line.to_string();
  };

  format!("{}{}{}", style.render(), line, style.render_reset())
}

pub fn unified_diff(path: &str, original: &[u8], formatted: &[u8]) -> String {
  let old_label = format!("a{path}");
  let new_label = format!("b{path}");
  let original_str = String::from_utf8_lossy(original);
  let formatted_str = String::from_utf8_lossy(formatted);

  TextDiff::from_lines(original_str.as_ref(), formatted_str.as_ref())
    .unified_diff()
    .context_radius(3)
    .header(&old_label, &new_label)
    .to_string()
}

pub fn print_colored_diff_to_stderr(diff: &str) -> Result<()> {
  let _guard = STDERR_LOCK
    .lock()
    .map_err(|err| anyhow::anyhow!("Failed to lock stderr: {err}"))?;
  let mut stderr = AutoStream::new(std::io::stderr(), ColorChoice::Auto);
  for line in diff.split_inclusive('\n') {
    stderr.write_all(style_line(line).as_bytes())?;
  }
  Ok(())
}
