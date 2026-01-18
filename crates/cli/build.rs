fn main() {
  let mut version = String::from("0.0.0-dev");
  if let Ok(version_env) = std::env::var("VERSION") {
    version = version_env.replace("v", "");
  }

  println!("cargo:rerun-if-env-changed=VERSION");
  println!("cargo:rustc-env=VERSION={version}");
}
