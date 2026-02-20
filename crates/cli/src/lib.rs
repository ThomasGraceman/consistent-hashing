//! CLI tool for managing consistent hash rings.
//!
//! Provides commands for:
//! - Inspecting ring state
//! - Adding/removing nodes
//! - Benchmarking performance
//! - Cluster management

pub mod commands;
pub mod config;

pub use commands::{Command, CommandResult};
pub use config::CliConfig;
