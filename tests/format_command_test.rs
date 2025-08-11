use anyhow::Result;

use pruner::{api::format::FormatOpts, commands::format::FormatContext};

mod common;

#[test]
fn format_command() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();

  let source = common::load_file("format_command/input.clj");

  let mut parser = tree_sitter::Parser::new();
  let result = pruner::commands::format::format_recursive(
    &mut parser,
    source.as_bytes(),
    FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    false,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
    },
  )
  .unwrap();

  let expected = common::load_file("format_command/output.clj");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}

#[test]
fn format_injections_only() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();

  let source = common::load_file("format_injections_only/input.clj");

  let mut parser = tree_sitter::Parser::new();
  let result = pruner::commands::format::format_recursive(
    &mut parser,
    source.as_bytes(),
    FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    true,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
    },
  )
  .unwrap();

  let expected = common::load_file("format_injections_only/output.clj");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}

#[test]
fn offset_dependent_printwidth() -> Result<()> {
  let grammars = common::grammars()?;
  let formatters = common::formatters();
  let languages = common::languages();

  let source = common::load_file("offset_dependent_printwidth/input.clj");

  let mut parser = tree_sitter::Parser::new();
  let result = pruner::commands::format::format_recursive(
    &mut parser,
    source.as_bytes(),
    FormatOpts {
      printwidth: 80,
      language: "clojure",
    },
    true,
    &FormatContext {
      grammars: &grammars,
      languages: &languages,
      formatters: &formatters,
    },
  )
  .unwrap();

  let expected = common::load_file("offset_dependent_printwidth/output.clj");

  assert_eq!(String::from_utf8(result).unwrap(), expected);

  Ok(())
}
