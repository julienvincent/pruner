use anyhow::{Context, Result};
use clap::ArgAction;
use rayon::prelude::*;
use std::{
  collections::HashSet,
  fs,
  io::Read,
  path::{Path, PathBuf},
  time::Instant,
};
use tree_sitter::Parser;

use crate::{
  api::{self, format::FormatOpts, grammar::Grammars},
  cli::GlobalOpts,
  config::{FormatterSpecs, LanguageFormatters, PrunerConfig},
};

#[derive(clap::Args, Debug)]
pub struct FormatArgs {
  #[arg(long)]
  lang: String,

  #[arg(long, default_value_t = 80)]
  print_width: u32,

  #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
  injected_regions_only: bool,
}

fn offset_lines(data: &mut Vec<u8>, offset: usize) {
  if offset == 0 {
    return;
  }

  let mut i = 0;
  while i < data.len() {
    if data[i] == b'\n' {
      let next = data.get(i + 1).copied();
      if matches!(next, Some(b'\n') | Some(b'\r') | None) {
        i += 1;
        continue;
      }
      let spaces = vec![b' '; offset];
      data.splice(i + 1..i + 1, spaces);
      i += offset + 1;
    } else {
      i += 1;
    }
  }
}

fn trim_trailing_whitespace(data: &mut Vec<u8>, preserve_newline: bool) {
  let mut removed_newline = false;
  while data.last() == Some(&b'\n') || data.last() == Some(&b'\r') {
    data.pop();
    removed_newline = true;
  }

  if preserve_newline && removed_newline {
    data.push(b'\n');
  }
}

fn column_for_byte(source: &[u8], byte_index: usize) -> usize {
  let target = byte_index.min(source.len());
  let line_start = source[..target]
    .iter()
    .rposition(|byte| *byte == b'\n')
    .map(|index| index + 1)
    .unwrap_or(0);

  target - line_start
}

fn min_leading_indent(text: &str) -> usize {
  let mut min_indent: Option<usize> = None;
  for line in text.lines() {
    if line.trim().is_empty() {
      continue;
    }
    let indent = line.chars().take_while(|ch| *ch == ' ').count();
    min_indent = Some(min_indent.map_or(indent, |current| current.min(indent)));
  }

  min_indent.unwrap_or(0)
}

fn strip_leading_indent(text: &str, indent: usize) -> String {
  if indent == 0 {
    return text.to_string();
  }

  let mut result = String::with_capacity(text.len());
  for segment in text.split_inclusive('\n') {
    let (line, newline) = if segment.ends_with('\n') {
      (&segment[..segment.len() - 1], "\n")
    } else {
      (segment, "")
    };
    let leading_spaces = line.chars().take_while(|ch| *ch == ' ').count();
    let trim_count = indent.min(leading_spaces);
    let trimmed = if trim_count > 0 {
      &line[trim_count..]
    } else {
      line
    };
    result.push_str(trimmed);
    result.push_str(newline);
  }

  result
}

fn sort_escape_chars(escape_chars: &HashSet<String>) -> Vec<String> {
  let mut chars: Vec<String> = escape_chars.iter().cloned().collect();
  chars.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
  chars
}

fn unescape_text(text: &str, escape_chars: &[String]) -> String {
  let mut result = text.to_string();
  for escape_char in escape_chars {
    let mut pattern = String::from("\\");
    pattern.push_str(escape_char);
    result = result.replace(&pattern, escape_char);
  }
  result
}

fn escape_text(text: &str, escape_chars: &[String]) -> String {
  let mut result = text.to_string();
  for escape_char in escape_chars {
    let mut replacement = String::from("\\");
    replacement.push_str(escape_char);
    result = result.replace(escape_char, &replacement);
  }
  result
}

pub struct FormatContext<'a> {
  pub grammars: &'a Grammars,
  pub languages: &'a LanguageFormatters,
  pub formatters: &'a FormatterSpecs,
}

pub fn format_recursive(
  parser: &mut Parser,
  source: &[u8],
  opts: FormatOpts,
  skip_root: bool,
  format_context: &FormatContext,
) -> Result<Vec<u8>> {
  let Some(grammar) = format_context.grammars.get(opts.language) else {
    return Ok(Vec::from(source));
  };

  let mut formatted_result = Vec::from(source);

  if !skip_root {
    if let Some(language_formatter_specs) = format_context.languages.get(opts.language) {
      if let Some(formatter_name) = language_formatter_specs.first() {
        if let Some(formatter) = format_context.formatters.get(formatter_name) {
          formatted_result = api::format::format(formatter, &formatted_result, &opts)?;
        }
      }
    }
  }

  let mut injected_regions =
    api::injections::extract_language_injections(parser, grammar, &formatted_result)?;
  // Sort in reverse order. File modifications can therefore be applied from end to start
  injected_regions.sort_by(|a, b| b.range.start_byte.cmp(&a.range.start_byte));

  let formatted_regions = injected_regions
    .par_iter()
    .map(|region| {
      let source_slice = &formatted_result[region.range.start_byte..region.range.end_byte];
      let escape_chars = sort_escape_chars(&region.opts.escape_chars);
      let source_str = String::from_utf8(Vec::from(source_slice))?;
      let unescaped_source_str = if escape_chars.is_empty() {
        source_str
      } else {
        unescape_text(&source_str, &escape_chars)
      };

      let mut indent = column_for_byte(source, region.range.start_byte);
      let mut normalized_source = unescaped_source_str;
      if indent > 0 {
        normalized_source = strip_leading_indent(&normalized_source, indent);
      } else {
        let min_indent = min_leading_indent(&normalized_source);
        if min_indent > 0 {
          normalized_source = strip_leading_indent(&normalized_source, min_indent);
          indent = min_indent;
        }
      }

      let unescaped_source = normalized_source.into_bytes();
      let adjusted_printwidth = opts.printwidth.saturating_sub(indent as u32);
      let mut parser = Parser::new();
      let mut formatted_sub_result = format_recursive(
        &mut parser,
        &unescaped_source,
        FormatOpts {
          printwidth: adjusted_printwidth.max(1),
          language: &region.lang,
        },
        false,
        format_context,
      )?;
      if !escape_chars.is_empty() {
        let formatted_str = String::from_utf8(formatted_sub_result)?;
        formatted_sub_result = escape_text(&formatted_str, &escape_chars).into_bytes();
      }
      let has_trailing_newline = source_slice.ends_with(b"\n");
      trim_trailing_whitespace(&mut formatted_sub_result, has_trailing_newline);
      offset_lines(&mut formatted_sub_result, indent);
      Ok((region.clone(), formatted_sub_result))
    })
    .collect::<Vec<Result<(api::injections::InjectedRegion, Vec<u8>)>>>();

  let mut region_results = Vec::with_capacity(formatted_regions.len());
  for result in formatted_regions {
    region_results.push(result?);
  }

  region_results.sort_by(|(a, _), (b, _)| b.range.start_byte.cmp(&a.range.start_byte));

  for (region, formatted_sub_result) in region_results {
    formatted_result.splice(
      region.range.start_byte..region.range.end_byte,
      formatted_sub_result,
    );
  }

  Ok(formatted_result)
}

fn paths_relative_to(root: &Path, paths: &[PathBuf]) -> Vec<PathBuf> {
  paths
    .iter()
    .cloned()
    .map(|entry| root.join(entry))
    .collect::<Vec<_>>()
}

pub fn handle(args: FormatArgs, global: GlobalOpts) -> Result<()> {
  let xdg_dirs = xdg::BaseDirectories::with_prefix("pruner");
  let config_path = global.config.or(xdg_dirs.find_config_file("config.toml"));
  let pruner_config = match config_path.as_deref() {
    Some(config_path) => PrunerConfig::from_file(config_path)
      .with_context(|| format!("Failed to load config {:?}", config_path))?,
    None => PrunerConfig::default(),
  };

  let cwd = std::env::current_dir()?;
  let repos_dir = cwd.join(
    pruner_config
      .grammar_download_dir
      .clone()
      .unwrap_or(xdg_dirs.place_data_file("grammars")?),
  );
  let lib_dir = cwd.join(
    pruner_config
      .grammar_build_dir
      .clone()
      .unwrap_or(xdg_dirs.place_data_file("build")?),
  );

  fs::create_dir_all(&repos_dir)?;
  fs::create_dir_all(&lib_dir)?;

  let grammars = pruner_config.grammars.clone().unwrap_or_default();

  let start = Instant::now();
  api::git::clone_all_grammars(&repos_dir, &grammars)?;
  log::debug!(
    "Grammar clone duration: {:?}",
    Instant::now().duration_since(start)
  );

  let config_relative_path = config_path
    .and_then(|path| path.parent().map(PathBuf::from))
    .unwrap_or(cwd.clone());
  let mut grammar_paths = paths_relative_to(
    &config_relative_path,
    &pruner_config.grammar_paths.unwrap_or_default(),
  );
  grammar_paths.push(repos_dir);

  let query_paths = paths_relative_to(
    &config_relative_path,
    &pruner_config.query_paths.unwrap_or_default(),
  );

  let start = Instant::now();
  let grammars = api::grammar::load_grammars(&grammar_paths, &query_paths, Some(lib_dir))
    .context("Failed to load grammars")?;
  log::debug!(
    "Grammar load duration: {:?}",
    Instant::now().duration_since(start)
  );

  let input = {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;
    buf
  };

  let mut parser = Parser::new();
  let start = Instant::now();
  let result = format_recursive(
    &mut parser,
    &input,
    FormatOpts {
      printwidth: args.print_width,
      language: &args.lang,
    },
    args.injected_regions_only,
    &FormatContext {
      grammars: &grammars,
      languages: &pruner_config.languages.unwrap_or_default(),
      formatters: &pruner_config.formatters.unwrap_or_default(),
    },
  )?;
  log::debug!(
    "Format time total: {:?}",
    Instant::now().duration_since(start)
  );

  print!("{}", String::from_utf8(result).unwrap());

  Ok(())
}
