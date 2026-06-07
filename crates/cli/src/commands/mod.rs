//! CLI commands for interacting with the consistent hash ring.

pub mod benchmark;
pub mod cluster;

use clap::Subcommand;

/// Available CLI subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Describe ring topology and ownership
    Describe {
        /// Number of nodes in the demo ring
        #[arg(short = 'n', long, default_value_t = 3)]
        nodes: usize,
        /// Virtual nodes per physical node
        #[arg(short = 'v', long, default_value_t = 256)]
        vnodes: usize,
    },
    /// Look up which node owns a key
    Lookup {
        /// Key to look up
        key: String,
        /// Number of nodes in the demo ring
        #[arg(short = 'n', long, default_value_t = 3)]
        nodes: usize,
        /// Virtual nodes per physical node
        #[arg(short = 'v', long, default_value_t = 256)]
        vnodes: usize,
        /// Number of replicas to show
        #[arg(short = 'r', long, default_value_t = 3)]
        replicas: usize,
    },
    /// Benchmark ring lookup throughput
    Benchmark {
        /// Number of nodes
        #[arg(short = 'n', long, default_value_t = 10)]
        nodes: usize,
        /// Virtual nodes per node
        #[arg(short = 'v', long, default_value_t = 256)]
        vnodes: usize,
        /// Number of lookup iterations
        #[arg(short = 'i', long, default_value_t = 100_000)]
        iterations: usize,
    },
    /// Cluster operations (add/remove nodes on a demo ring)
    AddNode {
        /// Node name
        name: String,
        /// Node ID (decimal u128)
        #[arg(long)]
        id: Option<u128>,
        /// Virtual nodes
        #[arg(short = 'v', long, default_value_t = 256)]
        vnodes: usize,
    },
    /// Remove a node from the demo ring
    RemoveNode {
        /// Node ID to remove
        id: u128,
    },
}

/// Result of executing a CLI command.
pub enum CommandResult {
    Success(String),
    Error(anyhow::Error),
}

impl Command {
    pub fn execute(self) -> CommandResult {
        match self {
            Command::Describe { nodes, vnodes } => cluster::describe(nodes, vnodes),
            Command::Lookup {
                key,
                nodes,
                vnodes,
                replicas,
            } => cluster::lookup(&key, nodes, vnodes, replicas),
            Command::Benchmark {
                nodes,
                vnodes,
                iterations,
            } => benchmark::run(nodes, vnodes, iterations),
            Command::AddNode { name, id, vnodes } => cluster::add_node(name, id, vnodes),
            Command::RemoveNode { id } => cluster::remove_node(id),
        }
    }
}
