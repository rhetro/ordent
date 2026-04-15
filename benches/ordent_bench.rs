use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use ordent::prelude::*;

/// Helper to set up the engine with an arbitrary number of nodes and edge density
fn setup_engine(num_nodes: usize, edges_per_node: usize) -> OrdentEngine {
    let num_edges = num_nodes * edges_per_node;
    let mut engine = OrdentEngine::new(num_edges, num_nodes);
    let mut nodes = Vec::with_capacity(num_nodes);

    for i in 0..num_nodes {
        nodes.push(engine.arena.insert(NodeState {
            theta: (i as u32).wrapping_mul(10),
            d_theta: 5,
        }));
    }
    engine.active_nodes.clone_from(&nodes);

    for i in 0..num_nodes {
        engine.target_nodes.push(nodes[i]);
        engine.edge_offsets.push((i * edges_per_node) as u32);
        engine.edge_counts.push(edges_per_node as u32);

        for j in 0..edges_per_node {
            let src_idx = (i + j) % num_nodes;
            engine.edge_sources.push(nodes[src_idx]);
            engine.edge_weights.push(0.5);
        }
    }

    // Warmup
    engine.tick(0);
    engine
}

fn bench_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("Ordent_Tick");

    // Verify scaling at 10,000, 50,000, and 100,000 nodes
    for &size in &[10_000, 50_000, 100_000] {
        // 1. Sparse graph: 1 edge per node
        let mut engine_sparse = setup_engine(size, 1);
        group.bench_with_input(
            BenchmarkId::new("Sparse (1 edge/node)", size),
            &size,
            |b, _| b.iter(|| engine_sparse.tick(black_box(16))),
        );

        // 2. Dense graph: 8 edges per node
        //   Fits perfectly into an f32x8 SIMD register, expected to yield maximum throughput
        let mut engine_dense = setup_engine(size, 8);
        group.bench_with_input(
            BenchmarkId::new("Dense (8 edges/node)", size),
            &size,
            |b, _| b.iter(|| engine_dense.tick(black_box(16))),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_tick);
criterion_main!(benches);
