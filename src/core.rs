use ordex::prelude::*;
use std::f32::consts::PI;
use std::simd::prelude::*;

pub type Bam32 = u32;

#[derive(Clone, Debug, Default)]
pub struct NodeState {
    pub theta: Bam32,
    pub d_theta: Bam32,
}

pub struct OrdentEngine {
    pub arena: OrdexArena<NodeState>,
    pub active_nodes: Vec<Index>,
    pub target_nodes: Vec<Index>,
    pub edge_offsets: Vec<u32>,
    pub edge_counts: Vec<u32>,
    pub edge_sources: Vec<Index>,
    pub edge_weights: Vec<f32>,

    flat_src_thetas: Vec<Bam32>,
    flat_dst_thetas: Vec<Bam32>,
    flat_forces: Vec<f32>,
    target_total_forces: Vec<Bam32>,
    verified_buffer: VerifiedIndices,
}

impl OrdentEngine {
    pub fn new(max_edges: usize, max_nodes: usize) -> Self {
        Self {
            arena: OrdexArena::new(),
            active_nodes: Vec::with_capacity(max_nodes),
            target_nodes: Vec::with_capacity(max_nodes),
            edge_offsets: Vec::with_capacity(max_nodes),
            edge_counts: Vec::with_capacity(max_nodes),
            edge_sources: Vec::with_capacity(max_edges),
            edge_weights: Vec::with_capacity(max_edges),

            flat_src_thetas: Vec::with_capacity(max_edges),
            flat_dst_thetas: Vec::with_capacity(max_edges),
            flat_forces: Vec::with_capacity(max_edges),
            target_total_forces: Vec::with_capacity(max_nodes),
            verified_buffer: VerifiedIndices::new(vec![]),
        }
    }

    pub fn tick(&mut self, dt_bam: Bam32) {
        // Phase 1: Integration
        for &idx in &self.active_nodes {
            let state = self.arena.get_mut(idx).unwrap();
            state.theta = state.theta.wrapping_add(state.d_theta.wrapping_mul(dt_bam));
        }

        let num_targets = self.target_nodes.len();
        let num_edges = self.edge_sources.len();
        if num_edges == 0 {
            return;
        }

        assert!(
            num_edges <= self.flat_src_thetas.capacity(),
            "Edge capacity exceeded!"
        );
        assert!(
            num_targets <= self.target_total_forces.capacity(),
            "Target capacity exceeded!"
        );

        // Phase 2: Gather
        unsafe {
            self.flat_src_thetas.set_len(num_edges);
            self.flat_dst_thetas.set_len(num_edges);
        }

        let mut edge_idx = 0;
        for i in 0..num_targets {
            let target_idx = self.target_nodes[i];
            let target_theta = self.arena.get(target_idx).unwrap().theta;

            let offset = self.edge_offsets[i] as usize;
            let count = self.edge_counts[i] as usize;

            for j in 0..count {
                let src_idx = self.edge_sources[offset + j];
                self.flat_src_thetas[edge_idx] = self.arena.get(src_idx).unwrap().theta;
                self.flat_dst_thetas[edge_idx] = target_theta;
                edge_idx += 1;
            }
        }

        // Phase 3: Compute
        unsafe {
            self.flat_forces.set_len(num_edges);
        }

        let chunks = num_edges / 8;
        let remainder_start = chunks * 8;

        for i in 0..chunks {
            let idx = i * 8;
            let src_simd = Simd::from_slice(&self.flat_src_thetas[idx..idx + 8]);
            let dst_simd = Simd::from_slice(&self.flat_dst_thetas[idx..idx + 8]);
            let weight_simd = Simd::from_slice(&self.edge_weights[idx..idx + 8]);

            let diff_bam = src_simd - dst_simd;
            let diff_f32 = bam_to_f32x8(diff_bam);
            let force = fast_sin_f32x8(diff_f32) * weight_simd;

            force.copy_to_slice(&mut self.flat_forces[idx..idx + 8]);
        }

        for i in remainder_start..num_edges {
            let src = self.flat_src_thetas[i];
            let dst = self.flat_dst_thetas[i];
            let weight = self.edge_weights[i];

            let diff_bam = src.wrapping_sub(dst);
            let diff_f32 = bam_to_f32_scalar(diff_bam);
            let force = fast_sin_f32_scalar(diff_f32) * weight;

            self.flat_forces[i] = force;
        }

        // Phase 4: Reduce
        unsafe {
            self.target_total_forces.set_len(num_targets);
        }

        for i in 0..num_targets {
            let offset = self.edge_offsets[i] as usize;
            let count = self.edge_counts[i] as usize;

            let mut sum = 0.0;
            for j in 0..count {
                sum += self.flat_forces[offset + j];
            }
            self.target_total_forces[i] = f32_to_bam(sum);
        }

        // Phase 5: Scatter via Ordex
        self.verified_buffer.clear_and_verify(&self.target_nodes);
        let mut force_iter = self.target_total_forces.iter();

        self.arena.ordex(&self.verified_buffer, |mut iter| {
            while let Some(state) = iter.next() {
                let force_bam = *force_iter.next().unwrap();
                state.theta = state.theta.wrapping_add(force_bam);
            }
        });
    }
}

// =========================================================
// Fast Math & BAM Utilities (Two's Complement Hack)
// =========================================================

#[inline(always)]
fn bam_to_f32x8(bam: Simd<u32, 8>) -> Simd<f32, 8> {
    // Casting the difference between two u32 values to i32 automatically wraps it into a signed ± range centered around 0.

    let diff_i32 = bam.cast::<i32>();
    let float_val = diff_i32.cast::<f32>();
    let scale = Simd::splat(PI / (i32::MAX as f32));
    float_val * scale
}

#[inline(always)]
fn bam_to_f32_scalar(bam: u32) -> f32 {
    let diff_i32 = bam as i32;
    let scale = PI / (i32::MAX as f32);
    (diff_i32 as f32) * scale
}

#[inline(always)]
fn f32_to_bam(val: f32) -> Bam32 {
    // Conversely, convert the f32 (radians) force back into BAM units.
    (val * ((i32::MAX as f32) / PI)) as i32 as u32
}

#[inline(always)]
fn fast_sin_f32x8(x: Simd<f32, 8>) -> Simd<f32, 8> {
    let x2 = x * x;
    let x3 = x2 * x;
    let x5 = x3 * x2;
    let c1 = Simd::splat(1.0 / 6.0);
    let c2 = Simd::splat(1.0 / 120.0);
    x - (x3 * c1) + (x5 * c2)
}

#[inline(always)]
fn fast_sin_f32_scalar(x: f32) -> f32 {
    let x2 = x * x;
    let x3 = x2 * x;
    let x5 = x3 * x2;
    x - (x3 / 6.0) + (x5 / 120.0)
}
