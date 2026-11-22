//! Reproducible wall-clock budget probe for native FIR automation.
//!
//! Run from frontend:
//! cargo run --offline --release -p verdant-plugins --no-default-features \
//!   --example native_fir_budget -- --ticks 1500 --frames 32 --json /tmp/native-fir-budget.json
//!
//! Setup is excluded. Every measured tick includes parameter writes and actual
//! stereo DSP. This is a manual measurement, with no timing assertions or claim
//! of a hard real-time guarantee on an arbitrary machine.
use std::{hint::black_box, path::PathBuf, time::Instant};
use verdant_plugins::builtin_dsp::{
    BandKind, EqParams, EqProcessor, MultibandProcessor, Placement, TableEffect, MULTIBAND_EXPERT_START, MULTIBAND_EXPERT_STRIDE,
};

const SAMPLE_RATE: f64 = 48_000.0;

struct Options {
    ticks: usize,
    frames: usize,
    json: Option<PathBuf>,
}
impl Options {
    fn parse() -> Result<Self, String> {
        let mut options = Self { ticks: 1500, frames: 32, json: None };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--ticks" => options.ticks = args.next().ok_or("--ticks needs a value")?.parse().map_err(|_| "invalid --ticks")?,
                "--frames" => options.frames = args.next().ok_or("--frames needs a value")?.parse().map_err(|_| "invalid --frames")?,
                "--json" => options.json = Some(args.next().ok_or("--json needs a path")?.into()),
                "--help" | "-h" => {
                    println!(
                        "native_fir_budget [--ticks 1500] [--frames 32] [--json PATH]\n\
                        48 kHz stereo; setup excluded; no timing threshold assertions."
                    );
                    std::process::exit(0);
                }
                _ => return Err(format!("unknown option: {arg}")),
            }
        }
        if !(100..=1_000_000).contains(&options.ticks) {
            return Err("--ticks must be 100..=1000000".into());
        }
        if !(1..=8192).contains(&options.frames) {
            return Err("--frames must be 1..=8192".into());
        }
        Ok(options)
    }
}

struct Measurement {
    name: &'static str,
    bands: usize,
    continuous_bands: usize,
    resolution: usize,
    writes: usize,
    elapsed_ms: f64,
    median_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    realtime_factor: f64,
    output_checksum: f64,
}
impl Measurement {
    fn json(&self) -> String {
        format!(
            "{{\"case\":\"{}\",\"bands\":{},\"continuous_bands\":{},\"resolution\":{},\
            \"parameter_writes_per_tick\":{},\"elapsed_ms\":{:.9},\"wall_median_ms\":{:.9},\
            \"wall_p95_ms\":{:.9},\"wall_max_ms\":{:.9},\"realtime_factor\":{:.9},\
            \"output_checksum\":{:.9}}}",
            self.name,
            self.bands,
            self.continuous_bands,
            self.resolution,
            self.writes,
            self.elapsed_ms,
            self.median_ms,
            self.p95_ms,
            self.max_ms,
            self.realtime_factor,
            self.output_checksum,
        )
    }
}

fn measure(
    options: &Options,
    name: &'static str,
    bands: usize,
    continuous_bands: usize,
    resolution: usize,
    writes: usize,
    mut run: impl FnMut(usize, &[f32], &mut [f32], &mut [f32]),
) -> Measurement {
    let input: Vec<f32> = (0..options.frames).map(|i| 0.1 * (i as f32 * 0.17).sin()).collect();
    let mut left = vec![0.0; options.frames];
    let mut right = left.clone();
    let mut samples = vec![0.0; options.ticks];
    let start = Instant::now();
    for (tick, elapsed) in samples.iter_mut().enumerate() {
        let tick_start = Instant::now();
        run(tick, &input, &mut left, &mut right);
        black_box((&left, &right));
        *elapsed = tick_start.elapsed().as_secs_f64() * 1000.0;
    }
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    samples.sort_by(f64::total_cmp);
    let audio_ms = options.frames as f64 * options.ticks as f64 / SAMPLE_RATE * 1000.0;
    let result = Measurement {
        name,
        bands,
        continuous_bands,
        resolution,
        writes,
        elapsed_ms,
        median_ms: samples[options.ticks / 2],
        p95_ms: samples[(options.ticks * 95).div_ceil(100) - 1],
        max_ms: samples[options.ticks - 1],
        realtime_factor: elapsed_ms / audio_ms,
        output_checksum: left.iter().chain(&right).map(|&x| x as f64).sum(),
    };
    println!(
        "{name}: bands={bands} continuous={continuous_bands} resolution={resolution} \
        writes={writes} elapsed={:.3}ms median={:.3}ms p95={:.3}ms max={:.3}ms RTF={:.3}",
        result.elapsed_ms, result.median_ms, result.p95_ms, result.max_ms, result.realtime_factor,
    );
    result
}

fn measure_eq(options: &Options, name: &'static str, bands: usize, continuous: bool, resolution: usize, writes: usize) -> Measurement {
    let mut processor = EqProcessor::new(SAMPLE_RATE);
    for band in &mut processor.params.bands {
        band.on = false;
    }
    for index in 0..bands {
        let band = &mut processor.params.bands[index];
        band.on = true;
        band.kind = BandKind::Bell;
        band.freq = 80.0 * 160.0_f64.powf(index as f64 / bands.saturating_sub(1).max(1) as f64);
        band.gain_db = if index % 2 == 0 { 3.0 } else { -3.0 };
        band.q = 0.8;
        band.place = if bands > 8 { Placement::ALL[index % 5] } else { Placement::Stereo };
        processor.params.slope_continuous[index] = continuous;
        processor.params.slope_db[index] = 18.3;
    }
    processor.params.phase = 2;
    processor.params.resolution = resolution;
    processor.refresh();
    // Match the host: static state is installed first, then audio admission
    // precedes frame-zero modulation. No initialization runs inside measurement.
    processor.begin_audio();
    let indexes: Vec<_> = (0..writes)
        .map(|index| {
            let field = if index % 2 == 0 { "freq" } else { "gain" };
            EqParams::index_of(&format!("b{}_{field}", index + 1)).unwrap()
        })
        .collect();
    measure(options, name, bands, if continuous { bands } else { 0 }, resolution, writes, |tick, input, left, right| {
        for (index, &parameter) in indexes.iter().enumerate() {
            let value = if index % 2 == 0 { 200.0 + (index * 300) as f64 + (tick % 11) as f64 } else { 2.0 + (tick % 11) as f64 * 0.1 };
            processor.set_index(parameter, black_box(value)).unwrap();
        }
        processor.process(input, input, left, right);
    })
}

fn measure_multiband(options: &Options) -> Measurement {
    let mut processor = MultibandProcessor::new(SAMPLE_RATE);
    processor.set("phase", 2.0).unwrap();
    processor.set("layout", 1.0).unwrap();
    for band in 0..6 {
        processor.set(&format!("b{}.low", band + 1), 100.0 + 200.0 * band as f64).unwrap();
        processor.set(&format!("b{}.high", band + 1), 2000.0 + 600.0 * band as f64).unwrap();
        processor.set(&format!("b{}.gain", band + 1), 2.0).unwrap();
    }
    processor.begin_audio();
    measure(options, "multiband6_write8", 6, 0, 0, 8, |tick, input, left, right| {
        for band in 0..6 {
            let index = MULTIBAND_EXPERT_START + band * MULTIBAND_EXPERT_STRIDE;
            processor.set_index(index, 100.0 + 200.0 * band as f64 + (tick % 11) as f64).unwrap();
            if band < 2 {
                processor.set_index(index + 1, 2000.0 + 600.0 * band as f64 + (tick % 11) as f64).unwrap();
            }
        }
        processor.process(input, input, None, left, right);
    })
}

fn main() -> Result<(), String> {
    let options = Options::parse()?;
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let deadline_ms = options.frames as f64 / SAMPLE_RATE * 1000.0;
    println!(
        "Native FIR budget: {profile}, {}-{}, sample_rate={SAMPLE_RATE}, \
        frames_per_tick={}, ticks={}, audio_ms={:.3}, deadline_per_tick_ms={deadline_ms:.6}",
        std::env::consts::ARCH,
        std::env::consts::OS,
        options.frames,
        options.ticks,
        options.frames as f64 * options.ticks as f64 / SAMPLE_RATE * 1000.0,
    );
    println!(
        "Wall clock includes OS scheduling; setup is excluded. RTF < 1 means faster than audio duration, not a hard real-time guarantee."
    );
    let measurements = [
        measure_eq(&options, "linear1_write1", 1, false, 1, 1),
        measure_eq(&options, "continuous1_write1", 1, true, 1, 1),
        measure_eq(&options, "linear8_write8", 8, false, 1, 8),
        measure_eq(&options, "continuous24_write8", 24, true, 2, 8),
        measure_multiband(&options),
    ];
    if let Some(path) = &options.json {
        let report = format!(
            "{{\n\"schema\":\"verdant.native_fir_budget.v1\",\n\"profile\":\"{profile}\",\
            \n\"architecture\":\"{}\",\"os\":\"{}\",\n\"sample_rate\":{SAMPLE_RATE},\
            \"frames_per_tick\":{},\"ticks\":{},\"deadline_per_tick_ms\":{deadline_ms:.9},\
            \n\"setup_excluded\":true,\"hard_realtime_guarantee\":false,\n\"measurements\":[\n{}\n]\n}}\n",
            std::env::consts::ARCH,
            std::env::consts::OS,
            options.frames,
            options.ticks,
            measurements.iter().map(Measurement::json).collect::<Vec<_>>().join(",\n"),
        );
        std::fs::write(path, report).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        println!("JSON report: {}", path.display());
    }
    Ok(())
}
