use ordent::prelude::*;
use std::f32::consts::PI;

/// [Edgecase 1] Proof of BAM Wrap-around
/// Tests whether nodes near 0 degrees and 360 degrees (2π) correctly interfere
/// with each other across the circular boundary using the "shortest path".
#[test]
fn test_edgecase_bam_wraparound() {
    let mut engine = OrdentEngine::new(2, 2);

    // Node 0: Near 2π (u32::MAX - small offset)
    let n0 = engine.arena.insert(NodeState {
        theta: u32::MAX - 100_000,
        d_theta: 0,
    });
    // Node 1: Near 0 (0 + small offset)
    let n1 = engine.arena.insert(NodeState {
        theta: 100_000,
        d_theta: 0,
    });

    engine.active_nodes = vec![n0, n1];
    // n1 is influenced by n0
    engine.target_nodes = vec![n1];
    engine.edge_offsets = vec![0];
    engine.edge_counts = vec![1];
    engine.edge_sources = vec![n0];
    engine.edge_weights = vec![1.0];

    engine.tick(16); // Advance time

    let state1 = engine.arena.get(n1).unwrap();

    // As a result of the attraction from n0 (-100k) from the perspective of n1 (100k),
    // n1 should cross the 0 boundary and wrap around to the negative domain (near u32::MAX).
    assert!(
        state1.theta > u32::MAX / 2,
        "Wrap-around interference failed! Target did not cross the 0 boundary."
    );
}

/// [Edgecase 2] Proof of Saddle Point (Singularity at π) and Synchronization Acceleration Noise
/// Proves that when phases are exactly opposite (π), the Taylor expansion distortion
/// generates a force (approx. 0.52) instead of strictly 0 (as in the exact Kuramoto model, sin(π)=0),
/// effectively breaking the equilibrium.
#[test]
fn test_edgecase_taylor_distortion_at_pi() {
    let mut engine = OrdentEngine::new(2, 2);

    let n0 = engine.arena.insert(NodeState {
        theta: u32::MAX / 2, // π
        d_theta: 0,
    });
    let n1 = engine.arena.insert(NodeState {
        theta: 0, // 0
        d_theta: 0,
    });

    engine.active_nodes = vec![n0, n1];
    engine.target_nodes = vec![n1];
    engine.edge_offsets = vec![0];
    engine.edge_counts = vec![1];
    engine.edge_sources = vec![n0];
    engine.edge_weights = vec![1.0];

    engine.tick(0);

    let state1 = engine.arena.get(n1).unwrap();

    // Taylor expansion at π: π - (π^3)/6 + (π^5)/120 ≈ 0.524
    let expected_distortion_f32 = PI - (PI.powi(3) / 6.0) + (PI.powi(5) / 120.0);
    let expected_force_bam = (expected_distortion_f32 * ((i32::MAX as f32) / PI)) as i32 as u32;

    let diff = state1.theta.abs_diff(expected_force_bam);
    // Verify that a force of approximately 0.52 (not 0) is applied, within a margin of error
    assert!(
        diff < u32::MAX / 100,
        "Taylor distortion at Pi did not produce the expected synchronization acceleration noise."
    );
}

/// [Edgecase 3] Robustness of CSR Topology (Multiple Sources & Isolated Nodes)
/// Tests whether multiple influences are correctly summed during the Reduce phase,
/// and ensures that isolated nodes (0 edges) do not crash the engine.
#[test]
fn test_edgecase_csr_topology() {
    let mut engine = OrdentEngine::new(10, 10);

    // Node 0: Fixed at π/2
    let n0 = engine.arena.insert(NodeState {
        theta: u32::MAX / 4,
        d_theta: 0,
    });
    // Node 1: Fixed at π/2
    let n1 = engine.arena.insert(NodeState {
        theta: u32::MAX / 4,
        d_theta: 0,
    });
    // Node 2: Influenced node (0)
    let n2 = engine.arena.insert(NodeState {
        theta: 0,
        d_theta: 0,
    });
    // Node 3: Isolated node (0 edges)
    let n3 = engine.arena.insert(NodeState {
        theta: 0,
        d_theta: 0,
    });

    engine.active_nodes = vec![n0, n1, n2, n3];

    // n2 is influenced by n0 and n1 (Count 2)
    // n3 is not influenced by anyone (Count 0)
    engine.target_nodes = vec![n2, n3];

    engine.edge_offsets = vec![0, 2];
    engine.edge_counts = vec![2, 0];
    engine.edge_sources = vec![n0, n1];
    engine.edge_weights = vec![1.0, 1.0];

    engine.tick(0);

    let state2 = engine.arena.get(n2).unwrap();
    let state3 = engine.arena.get(n3).unwrap();

    // n2 should receive a total force of 1.0 + 1.0 = 2.0 (Proof of Reduce)
    let single_force_bam = (1.0 * ((i32::MAX as f32) / PI)) as i32 as u32;
    let expected_double_force = single_force_bam.wrapping_add(single_force_bam);

    let diff = state2.theta.abs_diff(expected_double_force);
    assert!(
        diff < u32::MAX / 100,
        "CSR Reduce failed to sum multiple sources."
    );

    // n3 should remain at theta = 0 since it receives no interference (Proof of Count 0)
    assert_eq!(state3.theta, 0, "Isolated node received unexpected force.");
}
