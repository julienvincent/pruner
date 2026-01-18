use std::path::PathBuf;

use crate::commands::format::FormatArgs;

#[derive(Debug, clap::Args)]
pub struct GlobalOpts {
  #[clap(long, global = true)]
  pub log_level: Option<log::LevelFilter>,

  #[arg(long, global = true)]
  pub config: Option<PathBuf>,

  /// Use named profiles from the config file. Can be specified multiple times;
  /// profiles are applied in order.
  #[arg(long, global = true)]
  pub profile: Vec<String>,
}

#[derive(clap::Parser, Debug)]
#[command(name = "pruner", version = env!("VERSION"))]
pub struct Cli {
  #[clap(flatten)]
  pub global_opts: GlobalOpts,

  #[command(subcommand)]
  pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
  /// Format one or more files
  Format(FormatArgs),
}
