#![cfg(not(debug_assertions))]
use ordent::prelude::*;
use std::time::Instant;

/// ⚠️ Extreme Performance Test (Measuring DRAM bandwidth limit)
/// This test assumes optimizations via SIMD and inlining.
/// Run command: cargo test --release --test limit_test

#[test]
fn test_extreme_limit_performance() {
    // [Compiler Hack] Immediately reject if executed in debug mode
    #[cfg(debug_assertions)]
    {
        panic!(
            "\n🔥 WARNING: Performance test executed in debug mode.\n\
             Without SIMD optimization, this test will time out.\n\
             Please run with the following command:\n\
             cargo test --release --test limit_test\n"
        );
    }

    let num_nodes = 100_000;
    let edges_per_node = 5; // 5 interferences per node (total 500,000 edges)
    let num_edges = num_nodes * edges_per_node;

    let mut engine = OrdentEngine::new(num_edges, num_nodes);
    let mut nodes = Vec::with_capacity(num_nodes);

    // Initialize 100,000 nodes
    for i in 0..num_nodes {
        nodes.push(engine.arena.insert(NodeState {
            theta: (i as u32).wrapping_mul(100),
            d_theta: 10,
        }));
    }
    engine.active_nodes.clone_from(&nodes);

    // Build CSR topology (500,000 edges)
    for i in 0..num_nodes {
        engine.target_nodes.push(nodes[i]);
        engine.edge_offsets.push((i * edges_per_node) as u32);
        engine.edge_counts.push(edges_per_node as u32);

        for j in 0..edges_per_node {
            // Simulate influence from neighboring nodes
            let src_idx = (i + j + 1) % num_nodes;
            engine.edge_sources.push(nodes[src_idx]);
            engine.edge_weights.push(0.1);
        }
    }

    // Initial run to warm up the pipeline
    engine.tick(0);

    let iterations = 1000;
    let start = Instant::now();

    // Execute 1000 continuous frames (ticks)
    for _ in 0..iterations {
        engine.tick(16); // dt_bam = 16
    }

    let elapsed = start.elapsed();

    println!("=====================================");
    println!("🔥 Extreme Limit Test (100k Nodes, 500k Edges) 🔥");
    println!("Total Time for {} ticks: {:?}", iterations, elapsed);
    println!("Average Time per tick: {:?}", elapsed / iterations as u32);
    println!("=====================================");

    // Assertion requiring a single tick (interference calc of 500k edges) to finish in a few milliseconds
    assert!(
        elapsed.as_millis() < 10000,
        "Performance limit breached: took too long."
    );
}
