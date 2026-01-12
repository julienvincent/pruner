wit_bindgen::generate!({
  world: "pruner:pruner/pruner@1.0.0",
  path: "../../wit",
  pub_export_macro: true,
});

use exports::pruner::pruner::formatter::{FormatError, FormatOpts};

struct Component;

impl exports::pruner::pruner::formatter::Guest for Component {
  fn format(source: Vec<u8>, _opts: FormatOpts) -> Result<Vec<u8>, FormatError> {
    let mut start = 0;
    let mut end = source.len();

    while start < end && (source[start] == b'\n' || source[start] == b'\r') {
      start += 1;
    }

    while end > start && (source[end - 1] == b'\n' || source[end - 1] == b'\r') {
      end -= 1;
    }

    Ok(source[start..end].to_vec())
  }
}

export!(Component);

#[cfg(test)]
use exports::pruner::pruner::formatter::Guest;

#[test]
fn format_test() -> Result<(), FormatError> {
  let source = "\n\nabc\n\n\n";
  let result = Component::format(
    source.as_bytes().to_vec(),
    FormatOpts {
      print_width: 80,
      lang: "na".into(),
    },
  )?;
  assert_eq!(String::from_utf8_lossy(&result), "abc");
  Ok(())
}
