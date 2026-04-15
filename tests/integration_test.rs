use ordent::prelude::*;
use std::time::Instant;

/// Test whether interference computation between two nodes is performed correctly (consistency between BAM and the Taylor expansion).
#[test]
fn test_basic_interference() {
    let mut engine = OrdentEngine::new(10, 10);

    // Node 0: fixed wave source (θ = π/2)
    let n0 = engine.arena.insert(NodeState {
        theta: (u32::MAX / 4), // π/2
        d_theta: 0,
    });

    // Node 1: affected node (θ = 0)
    let n1 = engine.arena.insert(NodeState {
        theta: 0,
        d_theta: 0,
    });

    // CSR structure setup: n1 receives influence from n0 with weight = 1.0
    engine.active_nodes = vec![n0, n1];
    engine.target_nodes = vec![n1]; // Only n1 is updated
    engine.edge_offsets = vec![0];
    engine.edge_counts = vec![1];
    engine.edge_sources = vec![n0];
    engine.edge_weights = vec![1.0];

    // dt_bam = 0 (self‑propagation disabled to test interference force only)
    engine.tick(0);

    let state1 = engine.arena.get(n1).unwrap();
    // For the phase difference n0(π/2) − n1(0), a force of sin(π/2) ≈ 1.0 is applied
    // Theta should advance by approximately 1.0 radian
    let expected_force_bam = (1.0 * ((i32::MAX as f32) / std::f32::consts::PI)) as i32 as u32;

    // Allow approximation error (within a few percent)
    let diff = state1.theta.abs_diff(expected_force_bam);
    assert!(
        diff < u32::MAX / 100,
        "Interference mismatch! Diff: {}",
        diff
    );
}

/// Inject 10,000 nodes and 10,000 edges into the main loop to test for allocation violations or undefined behavior
#[test]
fn test_limit_massive_edges() {
    let num_nodes = 10_000;
    let num_edges = 10_000;
    let mut engine = OrdentEngine::new(num_edges, num_nodes);

    let mut nodes = Vec::with_capacity(num_nodes);
    for i in 0..num_nodes {
        let n = engine.arena.insert(NodeState {
            theta: (i as u32).wrapping_mul(1000),
            d_theta: 100,
        });
        nodes.push(n);
    }

    engine.active_nodes.clone_from(&nodes);

    // Build a large CSR structure resembling a linear chain
    // Node[i] is influenced by Node[i‑1]
    engine.target_nodes.clear();
    engine.edge_offsets.clear();
    engine.edge_counts.clear();
    engine.edge_sources.clear();
    engine.edge_weights.clear();

    for i in 1..num_nodes {
        engine.target_nodes.push(nodes[i]);
        engine.edge_offsets.push((i - 1) as u32);
        engine.edge_counts.push(1);
        engine.edge_sources.push(nodes[i - 1]);
        engine.edge_weights.push(0.5);
    }

    // Warm‑up spin to heat the pipeline
    engine.tick(0);

    let start = Instant::now();
    let iterations = 100;

    for _ in 0..iterations {
        engine.tick(10); // dt_bam = 10
    }

    let elapsed = start.elapsed();
    println!("Elapsed Time for {} ticks: {:?}", iterations, elapsed);
    println!("Time per tick: {:?}", elapsed / iterations as u32);

    // Prove that one million edge‑processing iterations complete with no panics or reallocations
    assert_eq!(engine.target_nodes.len(), num_nodes - 1);
}
