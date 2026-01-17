use pruner::config::PrunerConfig;
use std::{
  collections::HashMap,
  fs::{self, File},
  io::Write,
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};

fn unique_temp_dir() -> PathBuf {
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("time should be available")
    .as_nanos();
  let temp_dir = std::env::temp_dir().join(format!("pruner-test-{nanos}"));
  fs::create_dir_all(&temp_dir).expect("should create temp dir");
  temp_dir
}

#[test]
fn loads_config_and_absolutizes_paths() {
  let temp_dir = unique_temp_dir();
  let config_path = temp_dir.join("config.toml");

  let mut file = File::create(&config_path).expect("should create config file");
  writeln!(
    file,
    r#"
query_paths = ["queries"]
grammar_paths = ["grammars"]
grammar_download_dir = "downloads"
grammar_build_dir = "build"
"#
  )
  .expect("should write config file");

  let config = PrunerConfig::from_file(&config_path).expect("should load config");

  let query_paths = config.query_paths.expect("query_paths should be set");
  let grammar_paths = config.grammar_paths.expect("grammar_paths should be set");

  assert_eq!(query_paths.len(), 1);
  assert_eq!(grammar_paths.len(), 1);
  assert_eq!(query_paths[0], temp_dir.join("queries"));
  assert_eq!(grammar_paths[0], temp_dir.join("grammars"));

  assert_eq!(
    config
      .grammar_download_dir
      .expect("grammar_download_dir should be set"),
    temp_dir.join("downloads")
  );
  assert_eq!(
    config
      .grammar_build_dir
      .expect("grammar_build_dir should be set"),
    temp_dir.join("build")
  );
}

#[test]
fn merges_configs_with_overlay_priority() {
  let base = PrunerConfig {
    query_paths: Some(vec![PathBuf::from("base_query")]),
    grammar_paths: Some(vec![PathBuf::from("base_grammar")]),
    grammar_download_dir: Some(PathBuf::from("base_downloads")),
    grammar_build_dir: Some(PathBuf::from("base_build")),
    grammars: None,
    languages: Some(HashMap::from([
      ("markdown".to_string(), vec!["base_fmt".to_string()]),
      ("clojure".to_string(), vec!["base_clj".to_string()]),
    ])),
    formatters: Some(HashMap::from([
      (
        "a".to_string(),
        pruner::config::FormatterSpec {
          cmd: "a".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
      (
        "fmt".to_string(),
        pruner::config::FormatterSpec {
          cmd: "base".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
    ])),
    wasm_formatters: None,
  };

  let overlay = PrunerConfig {
    query_paths: Some(vec![PathBuf::from("overlay_query")]),
    grammar_paths: Some(vec![PathBuf::from("overlay_grammar")]),
    grammar_download_dir: Some(PathBuf::from("overlay_downloads")),
    grammar_build_dir: None,
    grammars: None,
    languages: Some(HashMap::from([
      ("markdown".to_string(), vec!["overlay_fmt".to_string()]),
      ("rust".to_string(), vec!["rust_fmt".to_string()]),
    ])),
    formatters: Some(HashMap::from([
      (
        "fmt".to_string(),
        pruner::config::FormatterSpec {
          cmd: "overlay".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
      (
        "b".to_string(),
        pruner::config::FormatterSpec {
          cmd: "b".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
    ])),
    wasm_formatters: None,
  };

  let merged = PrunerConfig::merge(&base, &overlay);

  assert_eq!(
    merged.query_paths.unwrap(),
    vec![PathBuf::from("base_query"), PathBuf::from("overlay_query")]
  );
  assert_eq!(
    merged.grammar_paths.unwrap(),
    vec![
      PathBuf::from("base_grammar"),
      PathBuf::from("overlay_grammar")
    ]
  );
  assert_eq!(
    merged.grammar_download_dir.unwrap(),
    PathBuf::from("overlay_downloads")
  );
  assert_eq!(
    merged.grammar_build_dir.unwrap(),
    PathBuf::from("base_build")
  );

  let formatters = merged.formatters.unwrap();
  assert_eq!(
    HashMap::from([
      (
        "a".to_string(),
        pruner::config::FormatterSpec {
          cmd: "a".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
      (
        "fmt".to_string(),
        pruner::config::FormatterSpec {
          cmd: "overlay".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
      (
        "b".to_string(),
        pruner::config::FormatterSpec {
          cmd: "b".to_string(),
          args: Vec::new(),
          stdin: None,
          fail_on_stderr: None,
        },
      ),
    ]),
    formatters
  );

  let languages = merged.languages.unwrap();
  assert_eq!(
    HashMap::from([
      ("clojure".to_string(), vec!["base_clj".to_string()]),
      ("markdown".to_string(), vec!["overlay_fmt".to_string()]),
      ("rust".to_string(), vec!["rust_fmt".to_string()]),
    ]),
    languages
  );
}
