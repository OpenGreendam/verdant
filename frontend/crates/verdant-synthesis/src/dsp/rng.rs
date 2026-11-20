//! NumPy 2.3.5 default_rng for the production u64 seed contract.
//!
//! SeedSequence(128-bit pool) → PCG XSL-RR 128/64 → double Ziggurat.
//! `standard_normal_vec(n)` is the same stream as n scalar draws; it never
//! substitutes a distribution or resets at phone/frame/block boundaries.
//! See numpy-rng-provenance.json for source hashes and NUMPY-LICENSE.txt.

#[path = "rng_tables.rs"]
mod tables;
use tables::{FI_DOUBLE, KI_DOUBLE, WI_DOUBLE, ZIGGURAT_NOR_INV_R, ZIGGURAT_NOR_R};

const PCG_MULTIPLIER: u128 = ((2549297995355413924_u128) << 64) | 4865540595714422341;

/// Owned NumPy default_rng state. No global generator or distribution cache.
#[derive(Clone, Debug)]
pub struct NumpyPcg64 {
    state: u128,
    increment: u128,
}

fn hashmix(mut value: u32, hash_const: &mut u32) -> u32 {
    value ^= *hash_const;
    *hash_const = hash_const.wrapping_mul(0x931e8875);
    value = value.wrapping_mul(*hash_const);
    value ^ (value >> 16)
}

fn mix(x: u32, y: u32) -> u32 {
    let value = 0xca01f9dd_u32.wrapping_mul(x).wrapping_sub(0x4973f715_u32.wrapping_mul(y));
    value ^ (value >> 16)
}

/// Unspawned SeedSequence(seed).generate_state(4, uint64), including NumPy's
/// variable-length little-endian integer entropy and uint32 wrapping arithmetic.
pub fn seed_sequence_state(seed: u64) -> [u64; 4] {
    let entropy = [seed as u32, (seed >> 32) as u32];
    let words = if seed >> 32 == 0 { 1 } else { 2 };
    let mut hash_const = 0x43b0d7e5;
    let mut pool = [0_u32; 4];
    for i in 0..4 {
        pool[i] = hashmix(if i < words { entropy[i] } else { 0 }, &mut hash_const);
    }
    for source in 0..4 {
        for destination in 0..4 {
            if source != destination {
                pool[destination] = mix(pool[destination], hashmix(pool[source], &mut hash_const));
            }
        }
    }
    let mut state = [0_u32; 8];
    hash_const = 0x8b51f9dd;
    for (i, word) in state.iter_mut().enumerate() {
        let mut value = pool[i % 4] ^ hash_const;
        hash_const = hash_const.wrapping_mul(0x58f38ded);
        value = value.wrapping_mul(hash_const);
        *word = value ^ (value >> 16);
    }
    std::array::from_fn(|i| u64::from(state[2*i]) | (u64::from(state[2*i+1]) << 32))
}

impl NumpyPcg64 {
    pub fn new(seed: u64) -> Self {
        let words = seed_sequence_state(seed);
        // pcg64.c packs the first uint64 as HIGH, unlike the uint32→uint64
        // little-endian pair conversion performed by SeedSequence itself.
        let initial_state = (u128::from(words[0]) << 64) | u128::from(words[1]);
        let sequence = (u128::from(words[2]) << 64) | u128::from(words[3]);
        let mut result = Self { state: 0, increment: sequence.wrapping_shl(1) | 1 };
        result.step();
        result.state = result.state.wrapping_add(initial_state);
        result.step();
        result
    }

    #[inline]
    fn step(&mut self) {
        self.state = self.state.wrapping_mul(PCG_MULTIPLIER).wrapping_add(self.increment);
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        // NumPy PCG64 emits from the POST-transition state. DXSM does not.
        self.step();
        let high = (self.state >> 64) as u64;
        (high ^ self.state as u64).rotate_right((self.state >> 122) as u32)
    }

    #[inline]
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    /// NumPy random_standard_normal (float64), including rejection/tail draws.
    /// The log1p branches intentionally match NumPy's 1-U policy and the tail
    /// sign uses rabs bit 8, not the sign bit of the fast rectangle proposal.
    pub fn standard_normal(&mut self) -> f64 {
        loop {
            let mut r = self.next_u64();
            let index = (r & 0xff) as usize;
            r >>= 8;
            let sign = r & 1;
            let rabs = (r >> 1) & 0x000f_ffff_ffff_ffff;
            let mut x = rabs as f64 * WI_DOUBLE[index];
            if sign != 0 { x = -x; }
            if rabs < KI_DOUBLE[index] { return x; }
            if index == 0 {
                loop {
                    let xx = -ZIGGURAT_NOR_INV_R * (-self.uniform()).ln_1p();
                    let yy = -(-self.uniform()).ln_1p();
                    if yy + yy > xx * xx {
                        return if (rabs >> 8) & 1 != 0 { -(ZIGGURAT_NOR_R + xx) } else { ZIGGURAT_NOR_R + xx };
                    }
                }
            }
            if (FI_DOUBLE[index - 1] - FI_DOUBLE[index]) * self.uniform() + FI_DOUBLE[index]
                < (-0.5 * x * x).exp()
            {
                return x;
            }
        }
    }

    pub fn standard_normal_vec(&mut self, count: usize) -> Vec<f64> {
        (0..count).map(|_| self.standard_normal()).collect()
    }
}
