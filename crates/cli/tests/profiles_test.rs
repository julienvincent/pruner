use pruner::config::{ConfigFile, LoadOpts, ProfileConfig};
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
fn apply_single_profile() {
  let base = ConfigFile {
    query_paths: Some(vec![PathBuf::from("base_queries")]),
    grammar_paths: Some(vec![PathBuf::from("base_grammars")]),
    grammar_download_dir: Some(PathBuf::from("base_downloads")),
    grammar_build_dir: Some(PathBuf::from("base_build")),
    languages: Some(HashMap::from([
      ("markdown".to_string(), vec!["prettier".into()]),
      ("rust".to_string(), vec!["rustfmt".into()]),
    ])),
    ..Default::default()
  };

  let profile = ProfileConfig {
    query_paths: Some(vec![PathBuf::from("profile_queries")]),
    grammar_download_dir: Some(PathBuf::from("profile_downloads")),
    languages: Some(HashMap::from([(
      "markdown".to_string(),
      vec!["profile_prettier".into()],
    )])),
    ..Default::default()
  };

  let result = base.apply_profile(&profile);

  assert_eq!(
    result.query_paths.unwrap(),
    vec![
      PathBuf::from("base_queries"),
      PathBuf::from("profile_queries")
    ]
  );
  assert_eq!(
    result.grammar_paths.unwrap(),
    vec![PathBuf::from("base_grammars")]
  );
  assert_eq!(
    result.grammar_download_dir.unwrap(),
    PathBuf::from("profile_downloads")
  );
  assert_eq!(
    result.grammar_build_dir.unwrap(),
    PathBuf::from("base_build")
  );

  let languages = result.languages.unwrap();
  assert_eq!(
    languages.get("markdown").unwrap(),
    &vec!["profile_prettier".into()]
  );
  assert_eq!(languages.get("rust").unwrap(), &vec!["rustfmt".into()]);
}

#[test]
fn apply_multiple_profiles_in_order() {
  let base = ConfigFile {
    query_paths: Some(vec![PathBuf::from("base")]),
    grammar_download_dir: Some(PathBuf::from("base_downloads")),
    languages: Some(HashMap::from([
      ("markdown".to_string(), vec!["base_md".into()]),
      ("rust".to_string(), vec!["base_rust".into()]),
      ("python".to_string(), vec!["base_python".into()]),
    ])),
    ..Default::default()
  };

  let profile_a = ProfileConfig {
    query_paths: Some(vec![PathBuf::from("profile_a")]),
    grammar_download_dir: Some(PathBuf::from("profile_a_downloads")),
    languages: Some(HashMap::from([
      ("markdown".to_string(), vec!["profile_a_md".into()]),
      ("rust".to_string(), vec!["profile_a_rust".into()]),
    ])),
    ..Default::default()
  };

  let profile_b = ProfileConfig {
    query_paths: Some(vec![PathBuf::from("profile_b")]),
    grammar_build_dir: Some(PathBuf::from("profile_b_build")),
    languages: Some(HashMap::from([(
      "markdown".to_string(),
      vec!["profile_b_md".into()],
    )])),
    ..Default::default()
  };

  let result = base.apply_profile(&profile_a).apply_profile(&profile_b);

  assert_eq!(
    result.query_paths.unwrap(),
    vec![
      PathBuf::from("base"),
      PathBuf::from("profile_a"),
      PathBuf::from("profile_b")
    ]
  );
  assert_eq!(
    result.grammar_download_dir.unwrap(),
    PathBuf::from("profile_a_downloads")
  );
  assert_eq!(
    result.grammar_build_dir.unwrap(),
    PathBuf::from("profile_b_build")
  );

  let languages = result.languages.unwrap();
  assert_eq!(
    languages.get("markdown").unwrap(),
    &vec!["profile_b_md".into()],
    "profile_b should override profile_a's markdown setting"
  );
  assert_eq!(
    languages.get("rust").unwrap(),
    &vec!["profile_a_rust".into()],
    "profile_a's rust setting should persist since profile_b doesn't override it"
  );
  assert_eq!(
    languages.get("python").unwrap(),
    &vec!["base_python".into()],
    "base python setting should persist since no profile overrides it"
  );
}

#[test]
fn profile_with_empty_fields_does_not_override() {
  let base = ConfigFile {
    query_paths: Some(vec![PathBuf::from("base_queries")]),
    grammar_paths: Some(vec![PathBuf::from("base_grammars")]),
    grammar_download_dir: Some(PathBuf::from("base_downloads")),
    grammar_build_dir: Some(PathBuf::from("base_build")),
    languages: Some(HashMap::from([(
      "markdown".to_string(),
      vec!["prettier".into()],
    )])),
    ..Default::default()
  };

  let empty_profile = ProfileConfig::default();

  let result = base.apply_profile(&empty_profile);

  assert_eq!(
    result.query_paths.unwrap(),
    vec![PathBuf::from("base_queries")]
  );
  assert_eq!(
    result.grammar_paths.unwrap(),
    vec![PathBuf::from("base_grammars")]
  );
  assert_eq!(
    result.grammar_download_dir.unwrap(),
    PathBuf::from("base_downloads")
  );
  assert_eq!(
    result.grammar_build_dir.unwrap(),
    PathBuf::from("base_build")
  );
  assert_eq!(
    result.languages.unwrap().get("markdown").unwrap(),
    &vec!["prettier".into()]
  );
}

#[test]
fn load_config_with_single_profile_from_toml() {
  let temp_dir = unique_temp_dir();
  let config_path = temp_dir.join("pruner.toml");

  let mut file = File::create(&config_path).expect("should create config file");
  writeln!(
    file,
    r#"
query_paths = ["queries"]

[languages]
markdown = ["prettier"]
rust = ["rustfmt"]

[profiles.ci]
query_paths = ["ci_queries"]

[profiles.ci.languages]
markdown = ["ci_prettier"]
"#
  )
  .expect("should write config file");

  std::env::set_current_dir(&temp_dir).expect("should change dir");

  let config = pruner::config::load(LoadOpts {
    config_path: Some(config_path),
    profiles: vec!["ci".into()],
  })
  .expect("should load config");

  assert_eq!(
    config.query_paths,
    vec![temp_dir.join("queries"), temp_dir.join("ci_queries")]
  );

  assert_eq!(
    config.languages.get("markdown").unwrap(),
    &vec!["ci_prettier".into()]
  );
  assert_eq!(
    config.languages.get("rust").unwrap(),
    &vec!["rustfmt".into()]
  );
}

#[test]
fn load_config_with_multiple_profiles_from_toml() {
  let temp_dir = unique_temp_dir();
  let config_path = temp_dir.join("pruner.toml");

  let mut file = File::create(&config_path).expect("should create config file");
  writeln!(
    file,
    r#"
query_paths = ["queries"]

[languages]
markdown = ["prettier"]
rust = ["rustfmt"]
python = ["black"]

[profiles.ci]
query_paths = ["ci_queries"]

[profiles.ci.languages]
markdown = ["ci_prettier"]
rust = ["ci_rustfmt"]

[profiles.debug]
query_paths = ["debug_queries"]

[profiles.debug.languages]
markdown = ["debug_prettier"]
"#
  )
  .expect("should write config file");

  std::env::set_current_dir(&temp_dir).expect("should change dir");

  let config = pruner::config::load(LoadOpts {
    config_path: Some(config_path),
    profiles: vec!["ci".into(), "debug".into()],
  })
  .expect("should load config");

  assert_eq!(
    config.query_paths,
    vec![
      temp_dir.join("queries"),
      temp_dir.join("ci_queries"),
      temp_dir.join("debug_queries")
    ]
  );

  assert_eq!(
    config.languages.get("markdown").unwrap(),
    &vec!["debug_prettier".into()],
    "debug profile should override ci profile's markdown"
  );
  assert_eq!(
    config.languages.get("rust").unwrap(),
    &vec!["ci_rustfmt".into()],
    "ci profile's rust should persist since debug doesn't override it"
  );
  assert_eq!(
    config.languages.get("python").unwrap(),
    &vec!["black".into()],
    "base python should persist since no profile overrides it"
  );
}

#[test]
fn load_config_with_nonexistent_profile_fails() {
  let temp_dir = unique_temp_dir();
  let config_path = temp_dir.join("pruner.toml");

  let mut file = File::create(&config_path).expect("should create config file");
  writeln!(
    file,
    r#"
query_paths = ["queries"]

[profiles.ci]
query_paths = ["ci_queries"]
"#
  )
  .expect("should write config file");

  std::env::set_current_dir(&temp_dir).expect("should change dir");

  let result = pruner::config::load(LoadOpts {
    config_path: Some(config_path),
    profiles: vec!["nonexistent".into()],
  });

  assert!(result.is_err());
  let err = result.unwrap_err();
  assert!(
    err.to_string().contains("Profile 'nonexistent' not found"),
    "Error message should mention the missing profile: {}",
    err
  );
}
