//! Filter building blocks shared by the built-in effects.

use std::f64::consts::TAU;

/// Normalized biquad coefficients `[b0, b1, b2, a1, a2]` (a0 = 1).
pub type Coefficients = [f64; 5];

pub const IDENTITY: Coefficients = [1.0, 0.0, 0.0, 0.0, 0.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rbj { LowPass, HighPass, Bell, LowShelf, HighShelf, Notch, BandPass }

/// RBJ Audio EQ Cookbook designs. Frequencies near Nyquist are clamped so
/// every design stays stable.
pub fn rbj(kind: Rbj, freq: f64, q: f64, gain_db: f64, sample_rate: f64) -> Coefficients {
    let f = freq.clamp(1.0, sample_rate * 0.49);
    let w0 = TAU * f / sample_rate;
    let (sin, cos) = w0.sin_cos();
    let alpha = sin / (2.0 * q.max(1e-3));
    let a = 10f64.powf(gain_db / 40.0);
    let (b0, b1, b2, a0, a1, a2) = match kind {
        Rbj::HighPass => ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        Rbj::LowPass => ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        Rbj::Bell => (1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a),
        Rbj::Notch => (1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        // Constant 0 dB peak gain.
        Rbj::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        Rbj::LowShelf => {
            let s = 2.0 * a.sqrt() * alpha;
            (a * ((a + 1.0) - (a - 1.0) * cos + s), 2.0 * a * ((a - 1.0) - (a + 1.0) * cos), a * ((a + 1.0) - (a - 1.0) * cos - s),
             (a + 1.0) + (a - 1.0) * cos + s, -2.0 * ((a - 1.0) + (a + 1.0) * cos), (a + 1.0) + (a - 1.0) * cos - s)
        }
        Rbj::HighShelf => {
            let s = 2.0 * a.sqrt() * alpha;
            (a * ((a + 1.0) + (a - 1.0) * cos + s), -2.0 * a * ((a - 1.0) + (a + 1.0) * cos), a * ((a + 1.0) + (a - 1.0) * cos - s),
             (a + 1.0) - (a - 1.0) * cos + s, 2.0 * ((a - 1.0) - (a + 1.0) * cos), (a + 1.0) - (a - 1.0) * cos - s)
        }
    };
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}

/// |H(e^jw)| of one biquad, in dB.
pub fn response_db([b0, b1, b2, a1, a2]: Coefficients, freq: f64, sample_rate: f64) -> f64 {
    let w = TAU * freq.clamp(1.0, sample_rate * 0.5) / sample_rate;
    let (c1, s1) = (w.cos(), w.sin());
    let (c2, s2) = ((2.0 * w).cos(), (2.0 * w).sin());
    let num = ((b0 + b1 * c1 + b2 * c2).powi(2) + (b1 * s1 + b2 * s2).powi(2)).sqrt();
    let den = ((1.0 + a1 * c1 + a2 * c2).powi(2) + (a1 * s1 + a2 * s2).powi(2)).sqrt();
    20.0 * (num / den.max(1e-12)).max(1e-12).log10()
}

/// Butterworth section Qs for a cut of `order` biquads (12 dB/oct each).
pub fn butterworth_qs(sections: usize) -> &'static [f64] {
    match sections {
        1 => &[std::f64::consts::FRAC_1_SQRT_2],
        2 => &[0.541_196_1, 1.306_563_0],
        3 => &[0.517_638_1, std::f64::consts::FRAC_1_SQRT_2, 1.931_851_7],
        _ => &[0.509_795_6, 0.601_344_9, 0.899_976_2, 2.562_915_4],
    }
}

/// Normalized-lattice biquad: the state energy cannot grow when the
/// coefficients change between samples, so automation never blows up.
#[derive(Clone, Copy, Debug)]
pub struct Lattice { k1: f64, k2: f64, t1: f64, t2: f64, b0: f64, c1: f64, c2: f64 }

impl Lattice {
    pub fn new([b0, b1, b2, a1, a2]: Coefficients) -> Self {
        // Schur reflection coefficients of the denominator. Stable poles give
        // |k1|, |k2| < 1; the t values normalize the lattice.
        let k1 = a1 / (1.0 + a2);
        let k2 = a2;
        let t1 = ((1.0 - k1) * (1.0 + k1)).sqrt();
        let t2 = ((1.0 - k2) * (1.0 + k2)).sqrt();
        let residual2 = b2 - b0 * a2;
        // Output taps recover the original numerator exactly in steady state.
        let c1 = (b1 - b0 * a1 - k1 * residual2) / (t1 * t2);
        let c2 = residual2 / t2;
        Self { k1, k2, t1, t2, b0, c1, c2 }
    }
}

impl Default for Lattice { fn default() -> Self { Self::new(IDENTITY) } }

#[derive(Clone, Copy, Default, Debug)]
pub struct LatticeState { s1: f64, s2: f64 }

impl LatticeState {
    #[inline]
    pub fn run(&mut self, c: &Lattice, x: f64) -> f64 {
        let y = c.b0 * x + c.c1 * self.s1 + c.c2 * self.s2;
        let reflected = c.t2 * x - c.k2 * self.s2;
        // An orthogonal rotation/reflection of (s1, reflected): with x = 0
        // the squared state norm becomes s1² + k2²·s2², so even
        // sample-by-sample changes cannot amplify stored energy.
        let s1 = -c.k1 * self.s1 + c.t1 * reflected;
        self.s2 = c.t1 * self.s1 + c.k1 * reflected;
        self.s1 = s1;
        y
    }
}

/// A fixed biquad with its own state (sidechain and post filters).
#[derive(Clone, Copy, Default, Debug)]
pub struct Biquad { pub c: Lattice, pub s: LatticeState }

impl Biquad {
    pub fn set(&mut self, c: Coefficients) { self.c = Lattice::new(c); }
    #[inline]
    pub fn run(&mut self, x: f64) -> f64 { self.s.run(&self.c, x) }
    pub fn reset(&mut self) { self.s = LatticeState::default(); }
}

/// One-pole smoothing coefficient for a time constant in milliseconds.
pub fn time_coefficient(ms: f64, sample_rate: f64) -> f64 { (-1.0 / (ms.max(0.001) * 0.001 * sample_rate)).exp() }

/// One-pole low-pass coefficient for a cutoff frequency.
pub fn one_pole_coefficient(freq: f64, sample_rate: f64) -> f64 { (-TAU * freq.clamp(1.0, sample_rate * 0.49) / sample_rate).exp() }

pub fn db_to_lin(db: f64) -> f64 { 10f64.powf(db / 20.0) }

pub fn lin_to_db(x: f64) -> f64 { if x > 1e-9 { 20.0 * x.log10() } else { -180.0 } }

/// Fixed-capacity sample delay line.
#[derive(Clone, Debug, Default)]
pub struct DelayLine { buf: Vec<f64>, head: usize }

impl DelayLine {
    pub fn new(capacity: usize) -> Self { Self { buf: vec![0.0; capacity.max(1)], head: 0 } }
    pub fn capacity(&self) -> usize { self.buf.len() }
    #[inline]
    pub fn push(&mut self, x: f64) {
        self.head += 1;
        if self.head == self.buf.len() { self.head = 0; }
        self.buf[self.head] = x;
    }
    /// The sample written `delay` pushes ago (0 = the latest).
    #[inline]
    pub fn tap(&self, delay: usize) -> f64 {
        let delay = delay.min(self.buf.len() - 1);
        let index = if delay <= self.head { self.head - delay } else { self.buf.len() + self.head - delay };
        self.buf[index]
    }
    /// Linear-interpolated fractional tap.
    #[inline]
    pub fn tap_frac(&self, delay: f64) -> f64 {
        let delay = delay.clamp(0.0, (self.buf.len() - 2) as f64);
        let whole = delay.floor();
        let frac = delay - whole;
        let a = self.tap(whole as usize);
        let b = self.tap(whole as usize + 1);
        a + (b - a) * frac
    }
    pub fn clear(&mut self) { self.buf.iter_mut().for_each(|x| *x = 0.0); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_line_taps_count_back_from_the_latest_sample() {
        let mut line = DelayLine::new(4);
        for x in 1..=6 { line.push(x as f64); }
        assert_eq!([line.tap(0), line.tap(1), line.tap(2), line.tap(3)], [6.0, 5.0, 4.0, 3.0]);
        assert_eq!(line.tap_frac(0.5), 5.5);
    }

    #[test]
    fn butterworth_cascade_is_minus_3_db_per_section_at_the_corner() {
        for sections in 1..=4 {
            let total: f64 = butterworth_qs(sections).iter()
                .map(|&q| response_db(rbj(Rbj::HighPass, 1000.0, q, 0.0, 48_000.0), 1000.0, 48_000.0)).sum();
            assert!((total + 3.01).abs() < 0.05, "{sections} sections: {total}");
        }
    }
}
