//! Cluster management commands.

use crate::commands::CommandResult;
use corelib::node::{Node, NodeId};
use corelib::ring::HashRing;
use corelib::topology::Topology;
use replication::{ReplicationStrategy, SimpleStrategy};
use std::time::{SystemTime, UNIX_EPOCH};

fn demo_ring(node_count: usize, vnodes: usize) -> HashRing {
    let ring = HashRing::new();
    for i in 0..node_count {
        let id = NodeId(i as u128 + 1);
        ring.add_node(Node::new(id, format!("node-{}", i + 1)), vnodes);
    }
    ring
}

pub fn describe(nodes: usize, vnodes: usize) -> CommandResult {
    let ring = demo_ring(nodes, vnodes);
    let topology = Topology::new(ring);
    CommandResult::Success(topology.describe())
}

pub fn lookup(key: &str, nodes: usize, vnodes: usize, replicas: usize) -> CommandResult {
    let ring = demo_ring(nodes, vnodes);
    let primary = ring.lookup(key.as_bytes());
    let replica_ids = ring.replicas_for_key(key.as_bytes(), replicas);

    let mut output = String::new();
    output.push_str(&format!("Key: {key}\n"));
    match primary {
        Some(id) => {
            let name = ring
                .get_node(&id)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| "unknown".into());
            output.push_str(&format!("Primary: {} ({name})\n", id.0));
        }
        None => output.push_str("Primary: (empty ring)\n"),
    }

    output.push_str("Replicas:\n");
    for (i, id) in replica_ids.iter().enumerate() {
        let name = ring
            .get_node(id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "unknown".into());
        output.push_str(&format!("  {}: {} ({name})\n", i + 1, id.0));
    }

    let strategy = SimpleStrategy::new(replicas);
    let strategy_replicas = strategy.replicas_for_key(&ring, key.as_bytes());
    output.push_str(&format!(
        "SimpleStrategy agrees: {}\n",
        strategy_replicas == replica_ids
    ));

    CommandResult::Success(output)
}

pub fn add_node(name: String, id: Option<u128>, vnodes: usize) -> CommandResult {
    let node_id = NodeId(id.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u128)
            .unwrap_or(1)
    }));

    let ring = HashRing::new();
    ring.add_node(Node::new(node_id, name.clone()), vnodes);

    CommandResult::Success(format!(
        "Added node '{}' (id={}, vnodes={}): {} tokens on ring",
        name,
        node_id.0,
        vnodes,
        ring.token_count()
    ))
}

pub fn remove_node(id: u128) -> CommandResult {
    let ring = demo_ring(3, 4);
    let node_id = NodeId(id);
    let removed = ring.remove_node(&node_id);

    if removed {
        CommandResult::Success(format!(
            "Removed node {}: {} nodes, {} tokens remain",
            id,
            ring.node_count(),
            ring.token_count()
        ))
    } else {
        CommandResult::Error(anyhow::anyhow!("node {id} not found on demo ring"))
    }
}
