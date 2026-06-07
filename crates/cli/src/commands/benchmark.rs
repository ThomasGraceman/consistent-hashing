//! Benchmark commands for measuring ring performance.

use crate::commands::CommandResult;
use corelib::node::{Node, NodeId};
use corelib::ring::HashRing;
use std::time::Instant;

pub fn run(nodes: usize, vnodes: usize, iterations: usize) -> CommandResult {
    let ring = HashRing::new();
    for i in 0..nodes {
        ring.add_node(Node::new(NodeId(i as u128 + 1), format!("node-{}", i + 1)), vnodes);
    }

    let start = Instant::now();
    for i in 0..iterations {
        let key = format!("benchmark-key-{}", i);
        let _ = ring.lookup(key.as_bytes());
    }
    let elapsed = start.elapsed();

    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;

    let output = format!(
        "Benchmark results:\n\
         Nodes: {nodes}\n\
         Vnodes per node: {vnodes}\n\
         Total tokens: {}\n\
         Iterations: {iterations}\n\
         Elapsed: {elapsed:?}\n\
         Throughput: {ops_per_sec:.0} lookups/sec\n\
         Latency: {ns_per_op:.1} ns/op",
        ring.token_count()
    );

    CommandResult::Success(output)
}
