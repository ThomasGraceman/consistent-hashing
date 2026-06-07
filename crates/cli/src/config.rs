//! CLI configuration and argument parsing.

use crate::commands::{Command, CommandResult};
use clap::Parser;

/// Consistent hash ring management CLI.
#[derive(Parser, Debug)]
#[command(name = "consistent-hash")]
#[command(about = "Manage and benchmark consistent hash rings", long_about = None)]
pub struct CliConfig {
    #[command(subcommand)]
    pub command: Command,
}

impl CliConfig {
    /// Run the selected subcommand.
    pub fn run(self) -> anyhow::Result<()> {
        match self.command.execute() {
            CommandResult::Success(msg) => {
                if !msg.is_empty() {
                    println!("{msg}");
                }
                Ok(())
            }
            CommandResult::Error(err) => Err(err),
        }
    }
}
