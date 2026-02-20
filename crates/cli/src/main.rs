//! CLI entry point for consistent-hash-rs.

use cli::{CliConfig, Command};
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let config = CliConfig::parse();
    config.run()
}
