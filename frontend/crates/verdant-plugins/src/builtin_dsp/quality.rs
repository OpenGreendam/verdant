//! Fixed-memory signal quality tools. Windowed-sinc resampling is independent
//! of any commercial implementation. Loudness follows the stereo BS.1770
//! K-weighting and EBU Tech 3341 windows/gates; histogram gating is quantized
//! to 0.01 LU. These algorithms are not a standards certification.
use super::filters::{Biquad, lin_to_db};
use std::{f64::consts::{PI, TAU}, sync::OnceLock};

pub const QUALITY_NAMES: [&str; 6] = ["关闭", "2×", "4×", "8×", "16×", "32×"];
pub const RESAMPLER_DELAY: usize = 32;
const TAPS: usize = 33;
const MAX_FACTOR: usize = 32;
const HIGH_TAPS: usize = (TAPS - 1) * MAX_FACTOR + 1;
type Kernels = [[f64; HIGH_TAPS]; 6];
static KERNELS: OnceLock<Kernels> = OnceLock::new();

fn kernels() -> &'static Kernels {
    KERNELS.get_or_init(|| {
        let mut all = [[0.0; HIGH_TAPS]; 6];
        for (power, kernel) in all.iter_mut().enumerate().skip(1) {
            let factor = 1usize << power;
            let end = (TAPS - 1) * factor;
            for (j, tap) in kernel.iter_mut().enumerate().take(end + 1) {
                let distance = (j as f64 - end as f64 / 2.0) / factor as f64;
                let sinc = if distance.abs() < 1e-12 { 1.0 } else { (PI * distance).sin() / (PI * distance) };
                let phase = TAU * j as f64 / end as f64;
                *tap = sinc * (0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos()) / factor as f64;
            }
            let sum: f64 = kernel[..=end].iter().sum();
            kernel[..=end].iter_mut().for_each(|v| *v /= sum);
        }
        all
    })
}

/// Polyphase interpolation plus matched FIR decimation. The exact nominal
/// latency is 32 base-rate frames at every active quality, zero when off.
/// All storage and shared coefficients are prepared by new(), never by run().
pub struct Oversampler {
    power: usize,
    input: [f64; TAPS],
    high: Box<[f64; HIGH_TAPS]>,
    input_head: usize,
    high_head: usize,
    coefficients: &'static Kernels,
}
impl Default for Oversampler { fn default() -> Self { Self::new() } }
impl Oversampler {
    pub fn new() -> Self { Self { power: 0, input: [0.0; TAPS], high: Box::new([0.0; HIGH_TAPS]), input_head: 0, high_head: 0, coefficients: kernels() } }
    pub fn set_quality(&mut self, power: usize) {
        let power = power.min(5);
        if power != self.power { self.power = power; self.reset(); }
    }
    pub fn factor(&self) -> usize { 1 << self.power }
    pub fn latency(&self) -> usize { if self.power == 0 { 0 } else { RESAMPLER_DELAY } }
    pub fn reset(&mut self) { self.input.fill(0.0); self.high.fill(0.0); self.input_head = 0; self.high_head = 0; }
    pub fn upsample(&mut self, x: f64, output: &mut [f64; MAX_FACTOR]) -> usize {
        if self.power == 0 { output[0] = x; return 1; }
        self.input[self.input_head] = x;
        let factor = self.factor(); let len = (TAPS - 1) * factor + 1;
        let kernel = &self.coefficients[self.power];
        for (phase, sample) in output.iter_mut().enumerate().take(factor) {
            *sample = 0.0;
            for k in 0..TAPS { let tap = k * factor + phase; if tap >= len { break; }
                *sample += self.input[(self.input_head + TAPS - k) % TAPS] * kernel[tap] * factor as f64;
            }
        }
        self.input_head = (self.input_head + 1) % TAPS;
        factor
    }
    pub fn downsample(&mut self, input: &[f64; MAX_FACTOR]) -> f64 {
        if self.power == 0 { return input[0]; }
        let factor = self.factor(); let len = (TAPS - 1) * factor + 1;
        let kernel = &self.coefficients[self.power]; let mut output = 0.0;
        for (phase, sample) in input.iter().enumerate().take(factor) {
            self.high[self.high_head] = *sample;
            if phase == 0 { for (k, coefficient) in kernel.iter().enumerate().take(len) {
                output += self.high[(self.high_head + HIGH_TAPS - k) % HIGH_TAPS] * coefficient;
            } }
            self.high_head = (self.high_head + 1) % HIGH_TAPS;
        }
        output
    }
    pub fn run(&mut self, x: f64, mut process: impl FnMut(f64) -> f64) -> f64 {
        let mut samples = [0.0; MAX_FACTOR]; let count = self.upsample(x, &mut samples);
        samples[..count].iter_mut().for_each(|x| *x = process(*x));
        self.downsample(&samples)
    }
}

/// Four-times bandlimited reconstruction peak. Audio alignment is 16 frames.
pub struct TruePeak { history: [f64; TAPS], head: usize, kernel: &'static [f64; HIGH_TAPS] }
impl Default for TruePeak { fn default() -> Self { Self::new() } }
impl TruePeak {
    pub fn new() -> Self { Self { history: [0.0; TAPS], head: 0, kernel: &kernels()[2] } }
    pub fn reset(&mut self) { self.history.fill(0.0); self.head = 0; }
    pub fn run(&mut self, x: f64) -> f64 { self.run_bounded(x).0 }
    /// The second value is a conservative absolute-coefficient bound, used
    /// by the strict ISP guard to remain safe even while its gain changes.
    pub fn run_bounded(&mut self, x: f64) -> (f64, f64) {
        self.history[self.head] = x;
        let mut maximum = x.abs(); let mut bound = x.abs();
        for phase in 0..4 {
            let mut value = 0.0; let mut absolute = 0.0;
            for k in 0..TAPS {
                let tap = k * 4 + phase; if tap > 128 { break; }
                let contribution = self.history[(self.head + TAPS - k) % TAPS] * self.kernel[tap] * 4.0;
                value += contribution; absolute += contribution.abs();
            }
            maximum = maximum.max(value.abs()); bound = bound.max(absolute);
        }
        self.head = (self.head + 1) % TAPS;
        (maximum, bound)
    }
}

/// A symmetric lowpass used for complementary linear-phase splits. The
/// residual is delayed dry minus low; therefore recombination is exact.
pub struct LinearSplit { taps: [f64; 129], history: [f64; 129], head: usize, frequency: f64, sample_rate: f64 }
impl Default for LinearSplit { fn default() -> Self { Self { taps: [0.0; 129], history: [0.0; 129], head: 0, frequency: 0.0, sample_rate: 0.0 } } }
impl LinearSplit {
    pub const LATENCY: usize = 64;
    pub fn configure(&mut self, frequency: f64, sample_rate: f64) {
        if frequency == self.frequency && sample_rate == self.sample_rate { return; }
        self.frequency = frequency; self.sample_rate = sample_rate;
        let cutoff = frequency.clamp(5.0, sample_rate * 0.48) / sample_rate;
        for (i, tap) in self.taps.iter_mut().enumerate() {
            let n = i as f64 - 64.0;
            let sinc = if n == 0.0 { 2.0 * cutoff } else { (TAU * cutoff * n).sin() / (PI * n) };
            let w = TAU * i as f64 / 128.0;
            *tap = sinc * (0.42 - 0.5 * w.cos() + 0.08 * (2.0 * w).cos());
        }
        let sum: f64 = self.taps.iter().sum();
        self.taps.iter_mut().for_each(|t| *t /= sum);
    }
    pub fn run(&mut self, x: f64) -> (f64, f64) {
        self.history[self.head] = x;
        let mut low = 0.0;
        for (k, tap) in self.taps.iter().enumerate() { low += self.history[(self.head + 129 - k) % 129] * tap; }
        let delayed = self.history[(self.head + 129 - Self::LATENCY) % 129];
        self.head = (self.head + 1) % 129;
        (low, delayed - low)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LoudnessReading {
    pub momentary_lufs: f64, pub short_term_lufs: f64, pub integrated_lufs: f64,
    pub maximum_momentary_lufs: f64, pub maximum_short_term_lufs: f64,
    #[serde(default)]
    pub loudness_range_lu: f64,
    pub true_peak_dbtp: [f64; 2], pub maximum_true_peak_dbtp: [f64; 2],
}
impl Default for LoudnessReading {
    fn default() -> Self { Self { momentary_lufs: -120.0, short_term_lufs: -120.0, integrated_lufs: -120.0,
        maximum_momentary_lufs: -120.0, maximum_short_term_lufs: -120.0, loudness_range_lu: 0.0,
        true_peak_dbtp: [-120.0; 2], maximum_true_peak_dbtp: [-120.0; 2] } }
}
const BINS: usize = 12001;
pub struct LoudnessMeter {
    filters: [[Biquad; 2]; 2],
    energy: Vec<f64>, head: usize, samples: u64, momentary_frames: usize, step: usize,
    sums: [f64; 2], histogram: Vec<(u64, f64)>, total: (u64, f64),
    short_histogram: Vec<(u64, f64)>, short_total: (u64, f64),
    peaks: [TruePeak; 2], interval_peaks: [f64; 2], reading: LoudnessReading, paused: bool,
}
fn energy_lufs(energy: f64) -> f64 { (-0.691 + 10.0 * energy.max(1e-30).log10()).max(-120.0) }
impl LoudnessMeter {
    pub fn new(sample_rate: f64) -> Self {
        let sample_rate = sample_rate.clamp(8000.0, 384000.0);
        let mut filters = [[Biquad::default(); 2]; 2];
        // Bilinear prewarping of the two BS.1770 K-weighting sections.
        let k = (PI * 1681.974450955533 / sample_rate).tan();
        let q = 0.7071752369554196; let vh = 10f64.powf(3.999843853973347 / 20.0); let vb = vh.powf(0.4996667741545416);
        let a = 1.0 + k / q + k * k;
        let shelf = [(vh + vb * k / q + k * k) / a, 2.0 * (k * k - vh) / a, (vh - vb * k / q + k * k) / a, 2.0 * (k * k - 1.0) / a, (1.0 - k / q + k * k) / a];
        let k = (PI * 38.13547087602444 / sample_rate).tan(); let q = 0.5003270373238773;
        let a = 1.0 + k / q + k * k;
        let high = [1.0, -2.0, 1.0, 2.0 * (k * k - 1.0) / a, (1.0 - k / q + k * k) / a];
        for channel in &mut filters { channel[0].set(shelf); channel[1].set(high); }
        Self { filters, energy: vec![0.0; (sample_rate * 3.0).round() as usize], head: 0, samples: 0,
            momentary_frames: (sample_rate * 0.4).round() as usize, step: (sample_rate * 0.1).round() as usize,
            sums: [0.0; 2], histogram: vec![(0, 0.0); BINS], total: (0, 0.0),
            short_histogram: vec![(0, 0.0); BINS], short_total: (0, 0.0), peaks: [TruePeak::new(), TruePeak::new()],
            interval_peaks: [0.0; 2], reading: LoudnessReading::default(), paused: false }
    }
    pub fn reading(&self) -> LoudnessReading { self.reading }
    pub fn set_paused(&mut self, paused: bool) { self.paused = paused; }
    pub fn reset(&mut self) {
        self.filters.iter_mut().flatten().for_each(Biquad::reset);
        self.energy.fill(0.0); self.histogram.fill((0, 0.0)); self.total = (0, 0.0);
        self.short_histogram.fill((0, 0.0)); self.short_total = (0, 0.0);
        self.head = 0; self.samples = 0; self.sums = [0.0; 2]; self.interval_peaks = [0.0; 2];
        self.peaks.iter_mut().for_each(TruePeak::reset); self.reading = LoudnessReading::default();
    }
    pub fn push(&mut self, left: f64, right: f64) {
        let mut energy = 0.0;
        for (c, sample) in [left, right].into_iter().enumerate() {
            let weighted = self.filters[c][0].run(sample);
            let weighted = self.filters[c][1].run(weighted);
            energy += weighted * weighted;
            self.interval_peaks[c] = self.interval_peaks[c].max(self.peaks[c].run(sample));
        }
        let len = self.energy.len();
        self.sums[0] += energy - self.energy[(self.head + len - self.momentary_frames) % len];
        self.sums[1] += energy - self.energy[self.head];
        self.energy[self.head] = energy;
        self.head = (self.head + 1) % len; self.samples += 1;
        if !self.samples.is_multiple_of(self.step as u64) { return; }
        let momentary = (self.sums[0] / self.momentary_frames as f64).max(0.0);
        self.reading.momentary_lufs = energy_lufs(momentary);
        self.reading.short_term_lufs = energy_lufs((self.sums[1] / len as f64).max(0.0));
        self.reading.maximum_momentary_lufs = self.reading.maximum_momentary_lufs.max(self.reading.momentary_lufs);
        self.reading.maximum_short_term_lufs = self.reading.maximum_short_term_lufs.max(self.reading.short_term_lufs);
        for c in 0..2 {
            self.reading.true_peak_dbtp[c] = lin_to_db(self.interval_peaks[c]).max(-120.0);
            self.reading.maximum_true_peak_dbtp[c] = self.reading.maximum_true_peak_dbtp[c].max(self.reading.true_peak_dbtp[c]);
        }
        self.interval_peaks = [0.0; 2];
        // EBU Tech 3342: three-second blocks, 100-millisecond hop, absolute -70
        // and relative -20 LU gates, then the 10th to 95th percentile span.
        if !self.paused && self.samples >= len as u64 && self.reading.short_term_lufs >= -70.0 {
            let bin = ((self.reading.short_term_lufs + 70.0) * 100.0).floor().clamp(0.0, (BINS - 1) as f64) as usize;
            let energy = (self.sums[1] / len as f64).max(0.0);
            self.short_histogram[bin].0 += 1; self.short_histogram[bin].1 += energy;
            self.short_total.0 += 1; self.short_total.1 += energy;
            let threshold = (energy_lufs(self.short_total.1 / self.short_total.0 as f64) - 20.0).max(-70.0);
            let first = ((threshold + 70.0) * 100.0).floor().clamp(0.0, (BINS - 1) as f64) as usize;
            let count: u64 = self.short_histogram[first..].iter().map(|v| v.0).sum();
            let quantile = |fraction: f64| {
                let rank = (count as f64 * fraction).ceil() as u64; let mut sum = 0;
                for (index, &(n, _)) in self.short_histogram.iter().enumerate().skip(first) { sum += n; if sum >= rank.max(1) { return index as f64 * 0.01 - 70.0; } }
                -70.0
            };
            self.reading.loudness_range_lu = (quantile(0.95) - quantile(0.10)).max(0.0);
        }
        if self.paused || self.samples < self.momentary_frames as u64 || self.reading.momentary_lufs < -70.0 { return; }
        let bin = ((self.reading.momentary_lufs + 70.0) * 100.0).floor().clamp(0.0, (BINS - 1) as f64) as usize;
        self.histogram[bin].0 += 1; self.histogram[bin].1 += momentary;
        self.total.0 += 1; self.total.1 += momentary;
        let gate = (energy_lufs(self.total.1 / self.total.0 as f64) - 10.0).max(-70.0);
        let first = ((gate + 70.0) * 100.0).floor().clamp(0.0, (BINS - 1) as f64) as usize;
        let (mut count, mut sum) = (0u64, 0.0);
        for &(n, e) in &self.histogram[first..] { count += n; sum += e; }
        self.reading.integrated_lufs = if count > 0 { energy_lufs(sum / count as f64) } else { -120.0 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversampling_has_fixed_latency_and_rejects_folded_harmonics() {
        for quality in 1..=5 {
            let mut sampler = Oversampler::new(); sampler.set_quality(quality);
            let mut peak = (0, 0.0);
            for i in 0..160 { let y = sampler.run(if i == 0 { 1.0 } else { 0.0 }, |x| x); if y.abs() > peak.1 { peak = (i, y.abs()); } }
            assert_eq!(peak.0, RESAMPLER_DELAY);
            assert!(peak.1 > 0.85);
        }
        let probe = |quality| {
            let mut s = Oversampler::new(); s.set_quality(quality);
            let mut re = 0.0; let mut im = 0.0;
            for n in 0..9600 {
                let x = 3.0 * (TAU * 10000.0 * n as f64 / 48000.0).sin();
                let y = s.run(x, f64::tanh);
                if n > 4800 { let p = TAU * 18000.0 * n as f64 / 48000.0; re += y * p.cos(); im += y * p.sin(); }
            }
            re.hypot(im)
        };
        assert!(probe(3) < probe(0) * 0.05);
    }
    #[test]
    fn reconstruction_detects_intersample_peaks() {
        let mut p = TruePeak::new(); let mut peak = 0f64;
        for n in 0..480 { peak = peak.max(p.run((TAU * 12000.0 * n as f64 / 48000.0 + PI / 4.0).sin())); }
        assert!(peak > 0.98);
    }
    #[test]
    fn loudness_stereo_alignment_gating_and_reset() {
        let mut meter = LoudnessMeter::new(48000.0);
        for n in 0..192000 { let x = 0.1 * (TAU * 1000.0 * n as f64 / 48000.0).sin(); meter.push(x, x); }
        let active = meter.reading();
        assert!((active.integrated_lufs + 20.0).abs() < 0.08, "{active:?}");
        assert!((active.momentary_lufs - active.short_term_lufs).abs() < 0.01);
        for _ in 0..192000 { meter.push(0.0, 0.0); }
        assert!((meter.reading().integrated_lufs - active.integrated_lufs).abs() < 0.3);
        assert!(meter.reading().momentary_lufs < -100.0);
        meter.reset(); assert_eq!(meter.reading(), LoudnessReading::default());
    }
    #[test]
    fn loudness_range_matches_two_level_ebu_test_signal() {
        let mut meter = LoudnessMeter::new(8000.0);
        for level in [0.1, 0.1 / 10f64.sqrt()] {
            for n in 0..160000 { let x = level * (TAU * 1000.0 * n as f64 / 8000.0).sin(); meter.push(x, x); }
        }
        for _ in 0..12000 { meter.push(0.0, 0.0); }
        assert!((meter.reading().loudness_range_lu - 10.0).abs() < 0.1, "{:?}", meter.reading());
    }


    #[test]
    fn integrated_loudness_starts_at_first_complete_four_hundred_millisecond_block() {
        let mut meter=LoudnessMeter::new(48000.0);
        for n in 0..4800 {let x=0.1*(TAU*1000.0*n as f64/48000.0).sin();meter.push(x,x);}
        assert!(meter.reading().momentary_lufs>-30.0);
        assert_eq!(meter.reading().integrated_lufs,-120.0);
        for n in 4800..19200 {let x=0.1*(TAU*1000.0*n as f64/48000.0).sin();meter.push(x,x);}
        assert!((meter.reading().integrated_lufs+20.0).abs()<0.1,"{:?}",meter.reading());
    }

}
