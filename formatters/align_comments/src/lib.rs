use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

wit_bindgen::generate!({
  world: "pruner:pruner/pruner@1.0.0",
  path: "../../wit",
  pub_export_macro: true,
});

use exports::pruner::pruner::formatter::{FormatError, FormatOpts};

struct Component;

#[derive(Debug, Clone, Copy)]
struct CommentInfo {
  start_byte: usize,
  start_row: usize,
  start_col: usize,
}

impl CommentInfo {
  fn from_node(node: &Node) -> Self {
    Self {
      start_byte: node.start_byte(),
      start_row: node.start_position().row,
      start_col: node.start_position().column,
    }
  }
}

fn is_double_comment(source: &[u8], comment: &CommentInfo) -> bool {
  let start = comment.start_byte;
  if start + 2 <= source.len() {
    &source[start..start + 2] == b";;"
  } else {
    false
  }
}

fn get_next_named_sibling_col(node: &Node) -> Option<usize> {
  let next = node.next_named_sibling()?;
  let next_row = next.start_position().row;
  let comment_row = node.start_position().row;

  if next_row == comment_row {
    return None;
  }

  Some(next.start_position().column)
}

fn get_prev_named_sibling_row(node: &Node) -> Option<usize> {
  let prev = node.prev_named_sibling()?;
  Some(prev.start_position().row)
}

fn is_same_node(a: &Node, b: &Node) -> bool {
  a.start_byte() == b.start_byte() && a.end_byte() == b.end_byte()
}

fn group_related_comments<'a>(comments: &[Node<'a>]) -> Vec<Vec<Node<'a>>> {
  let mut groups: Vec<Vec<Node<'a>>> = Vec::new();
  let mut current_group: Vec<Node<'a>> = Vec::new();

  for (i, comment) in comments.iter().enumerate() {
    if current_group.is_empty() {
      current_group.push(*comment);
    } else {
      let prev = &comments[i - 1];

      let prev_sibling = comment.prev_named_sibling();
      let next_sibling = comment.next_named_sibling();

      let prev_same_group = prev_sibling.map_or(false, |s| is_same_node(&s, prev));
      let next_same_group = next_sibling.map_or(false, |s| is_same_node(&s, prev));

      if prev_same_group || next_same_group {
        current_group.push(*comment);
      } else {
        groups.push(current_group);
        current_group = vec![*comment];
      }
    }
  }

  if !current_group.is_empty() {
    groups.push(current_group);
  }

  groups
}

struct Edit {
  start_byte: usize,
  end_byte: usize,
  replacement: Vec<u8>,
}

fn align_comment_group(source: &[u8], group: &[CommentInfo], target_col: usize) -> Vec<Edit> {
  let mut edits = Vec::new();

  for comment in group.iter() {
    let current_col = comment.start_col;

    if current_col == target_col {
      continue;
    }

    let line_start = find_line_start(source, comment.start_byte);
    let expected_indent = " ".repeat(target_col);

    edits.push(Edit {
      start_byte: line_start,
      end_byte: comment.start_byte,
      replacement: expected_indent.into_bytes(),
    });
  }

  edits
}

fn find_line_start(source: &[u8], pos: usize) -> usize {
  let mut start = pos;
  while start > 0 && source[start - 1] != b'\n' {
    start -= 1;
  }
  start
}

fn apply_edits(source: Vec<u8>, mut edits: Vec<Edit>) -> Vec<u8> {
  edits.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

  let mut result = source;
  for edit in edits {
    let before = &result[..edit.start_byte];
    let after = &result[edit.end_byte..];
    result = [before, &edit.replacement, after].concat();
  }

  result
}

impl exports::pruner::pruner::formatter::Guest for Component {
  fn format(source: Vec<u8>, _opts: FormatOpts) -> Result<Vec<u8>, FormatError> {
    let mut parser = Parser::new();
    let language = tree_sitter_clojure::LANGUAGE.into();

    parser
      .set_language(&language)
      .map_err(|err| FormatError::Error(err.to_string()))?;

    let tree = parser
      .parse(&source, None)
      .ok_or_else(|| FormatError::Error("Parse returned None".into()))?;

    let query = Query::new(&language, "(comment) @comment")
      .map_err(|err| FormatError::Error(err.to_string()))?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches::<&[u8], &[u8]>(&query, tree.root_node(), &source);

    let mut comment_nodes: Vec<Node> = Vec::new();

    while let Some(query_match) = matches.next() {
      for capture in query_match.captures {
        let node = capture.node;
        let info = CommentInfo::from_node(&node);

        if !is_double_comment(&source, &info) {
          continue;
        }

        if let Some(prev_row) = get_prev_named_sibling_row(&node) {
          if prev_row == info.start_row {
            continue;
          }
        }

        comment_nodes.push(node);
      }
    }

    if comment_nodes.is_empty() {
      return Ok(source);
    }

    let groups = group_related_comments(&comment_nodes);

    let mut all_edits: Vec<Edit> = Vec::new();

    for group in &groups {
      if group.is_empty() {
        continue;
      }

      let last_node = &group[group.len() - 1];
      let last_info = CommentInfo::from_node(last_node);

      let target_col = get_next_named_sibling_col(last_node).unwrap_or(last_info.start_col);

      let group_infos: Vec<CommentInfo> = group.iter().map(CommentInfo::from_node).collect();
      let edits = align_comment_group(&source, &group_infos, target_col);
      all_edits.extend(edits);
    }

    let result = apply_edits(source, all_edits);
    Ok(result)
  }
}

export!(Component);

#[cfg(test)]
use exports::pruner::pruner::formatter::Guest;

#[test]
fn test_align_single_comment() -> Result<(), FormatError> {
  let source = r#"
(defn foo []
;; This is a comment
  (println "hello"))"#;

  let result = Component::format(
    source.as_bytes().to_vec(),
    FormatOpts {
      print_width: 80,
      lang: "clojure".into(),
    },
  )?;

  let expected = r#"
(defn foo []
  ;; This is a comment
  (println "hello"))"#;

  assert_eq!(String::from_utf8_lossy(&result), expected);
  Ok(())
}

#[test]
fn test_align_multiple_connected_comments() -> Result<(), FormatError> {
  let source = r#"
(defn foo []
;; Comment 1
;; Comment 2
  (println "hello"))"#;

  let result = Component::format(
    source.as_bytes().to_vec(),
    FormatOpts {
      print_width: 80,
      lang: "clojure".into(),
    },
  )?;

  let expected = r#"
(defn foo []
  ;; Comment 1
  ;; Comment 2
  (println "hello"))"#;

  assert_eq!(String::from_utf8_lossy(&result), expected);
  Ok(())
}

#[test]
fn test_no_change_when_aligned() -> Result<(), FormatError> {
  let source = r#"
(defn foo []
  ;; Already aligned
  (println "hello"))"#;

  let result = Component::format(
    source.as_bytes().to_vec(),
    FormatOpts {
      print_width: 80,
      lang: "clojure".into(),
    },
  )?;

  assert_eq!(String::from_utf8_lossy(&result), source);
  Ok(())
}

#[test]
fn test_ignore_single_semicolon() -> Result<(), FormatError> {
  let source = r#"
(defn foo []
; Single semicolon comment
  (println "hello"))"#;

  let result = Component::format(
    source.as_bytes().to_vec(),
    FormatOpts {
      print_width: 80,
      lang: "clojure".into(),
    },
  )?;

  assert_eq!(String::from_utf8_lossy(&result), source);
  Ok(())
}

#[test]
fn test_ignore_inline_comment() -> Result<(), FormatError> {
  let source = r#"
(defn foo [] ;; inline comment
  (println "hello"))"#;

  let result = Component::format(
    source.as_bytes().to_vec(),
    FormatOpts {
      print_width: 80,
      lang: "clojure".into(),
    },
  )?;

  assert_eq!(String::from_utf8_lossy(&result), source);
  Ok(())
}

#[test]
fn test_align_multiple_unconnected_comments() -> Result<(), FormatError> {
  let source = r#"
(defn foo []
;; Comment 1
;; Comment 2
  (println "hello")
    ;; Comment 3
  (let [a 1
    ;; Comment 4
        b 2
          ;; Comment 5
        c 3]))"#;

  let result = Component::format(
    source.as_bytes().to_vec(),
    FormatOpts {
      print_width: 80,
      lang: "clojure".into(),
    },
  )?;

  let expected = r#"
(defn foo []
  ;; Comment 1
  ;; Comment 2
  (println "hello")
  ;; Comment 3
  (let [a 1
        ;; Comment 4
        b 2
        ;; Comment 5
        c 3]))"#;

  assert_eq!(String::from_utf8_lossy(&result), expected);
  Ok(())
}
