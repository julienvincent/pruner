use std::path::PathBuf;

use crate::commands::format::FormatArgs;

#[derive(Debug, clap::Args)]
pub struct GlobalOpts {
  #[clap(long, global = true)]
  pub log_level: Option<log::LevelFilter>,

  #[arg(long)]
  pub config: Option<PathBuf>,
}

#[derive(clap::Parser, Debug)]
#[command(name = "prune", version = env!("VERSION"))]
pub struct Cli {
  #[clap(flatten)]
  pub global_opts: GlobalOpts,

  #[command(subcommand)]
  pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
  /// Format a file
  Format(FormatArgs),
}
