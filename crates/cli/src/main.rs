//! CLI entry point for consistent-hash-rs.

use clap::Parser;
use cli::CliConfig;

fn main() -> anyhow::Result<()> {
    let config = CliConfig::parse();
    config.run()
}
