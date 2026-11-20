//! Banded Gaussian normal equations, original 41-stream generation and
//! three forward passes with the original float32 frame stores.
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use super::duration::pairwise_sum;

const WINDOWS: [&[f64];3] = [&[1.0],&[-0.5,0.0,0.5],&[0.25,-0.5,0.25]];
#[derive(Clone)]
struct SymmetricBand { size: usize, width: usize, lower: Vec<f64> }
impl SymmetricBand {
    fn new(size: usize, width: usize) -> Result<Self> {
        let width = width.min(size.saturating_sub(1));
        let length = size.checked_mul(width+1).context("MLPG matrix size overflow")?;
        Ok(Self { size,width,lower:vec![0.0;length] })
    }
    fn get(&self, row: usize, column: usize) -> f64 {
        let (row,column) = if row >= column { (row,column) } else { (column,row) };
        if row-column > self.width { 0.0 } else { self.lower[row*(self.width+1)+row-column] }
    }
    fn set(&mut self, row: usize, column: usize, value: f64) {
        self.lower[row*(self.width+1)+row-column] = value;
    }
    fn add(&mut self, row: usize, column: usize, value: f64) -> Result<()> {
        ensure!(row >= column && row-column <= self.width, "MLPG normal matrix escaped its declared band");
        let index = row*(self.width+1)+row-column;
        self.lower[index] += value;
        ensure!(self.lower[index].is_finite(), "nonfinite MLPG normal equation");
        Ok(())
    }
    fn nonzeros(&self) -> usize {
        let mut count = 0;
        for row in 0..self.size {
            for column in row.saturating_sub(self.width)..=row {
                if self.get(row,column) != 0.0 { count += if row == column { 1 } else { 2 }; }
            }
        }
        count
    }
    fn multiply(&self, values: &[f64]) -> Vec<f64> {
        let mut result = vec![0.0;self.size];
        for row in 0..self.size {
            for column in row.saturating_sub(self.width)..=row {
                let value = self.get(row,column);
                result[row] += value*values[column];
                if column != row { result[column] += value*values[row]; }
            }
        }
        result
    }
    fn solve(&self, rhs: &[f64]) -> Result<Vec<f64>> {
        // Positive static precision guarantees SPD. LDL^T keeps the exact
        // band; no dense utterance matrix, diagonal floor or regularizer.
        let mut factors = self.clone();
        for row in 0..self.size {
            let begin = row.saturating_sub(self.width);
            for column in begin..row {
                let mut value = self.get(row,column);
                for k in begin.max(column.saturating_sub(self.width))..column {
                    value -= factors.get(row,k)*factors.get(k,k)*factors.get(column,k);
                }
                let factor = value/factors.get(column,column);
                ensure!(factor.is_finite(), "nonfinite MLPG factorization");
                factors.set(row,column,factor);
            }
            let mut diagonal = self.get(row,row);
            for column in begin..row {
                let l = factors.get(row,column);
                diagonal -= l*l*factors.get(column,column);
            }
            ensure!(diagonal.is_finite() && diagonal > 0.0,
                "MLPG normal equation is not numerically positive definite");
            factors.set(row,row,diagonal);
        }
        let mut result = rhs.to_vec();
        for row in 0..self.size {
            for column in row.saturating_sub(self.width)..row {
                result[row] -= factors.get(row,column)*result[column];
            }
        }
        for (row,value) in result.iter_mut().enumerate() { *value /= factors.get(row,row); }
        for row in (0..self.size).rev() {
            for column in row+1..(row+self.width+1).min(self.size) {
                result[row] -= factors.get(column,row)*result[column];
            }
        }
        ensure!(result.iter().all(|v| v.is_finite()), "nonfinite MLPG solution");
        Ok(result)
    }
}

// Replicated endpoint columns combine before any Gaussian weight is applied,
// exactly as scipy.sparse COO -> CSR in temporal_matrix.
fn temporal_terms(frame: usize, frames: usize, window: &[f64]) -> ([(usize,f64);3],usize) {
    let mut terms = [(0,0.0);3];
    let mut used = 0;
    let center = window.len()/2;
    for (index,&value) in window.iter().enumerate() {
        if value == 0.0 { continue; }
        let column = (frame as isize+index as isize-center as isize).clamp(0,frames as isize-1) as usize;
        if used > 0 && terms[used-1].0 == column { terms[used-1].1 += value; }
        else { terms[used] = (column,value); used += 1; }
    }
    (terms,used)
}

#[derive(Default)]
struct Diagnostics { unknowns: usize, nonzeros: usize, residual_square: f64, rhs_square: f64, objective: f64 }
impl Diagnostics {
    fn report(&self, drop_dynamic: bool) -> Value {
        json!({"solver":"Rust symmetric banded LDL^T (no pivot or variance floor)",
            "unknowns":self.unknowns,"normal_matrix_nonzeros":self.nonzeros,
            "relative_normal_equation_residual":self.residual_square.sqrt()/self.rhs_square.sqrt().max(1e-30),
            "weighted_squared_error":self.objective,"temporal_windows":WINDOWS,
            "temporal_window_evidence":"original RVA0x4f9d0/0x4fe50; constants RVA0x11a610/0x11a640",
            "boundary_policy":if drop_dynamic { "ignore dynamic observations at first and last timeline frame" }
                else { "replicate first and last frame; legacy reconstruction" },
            "boundary_evidence":if drop_dynamic { "original RVA0x4f140/0x4f090" } else { "legacy assumption" },
            "variance_floor_added":false})
    }
    fn collect(&mut self, normal: &SymmetricBand, rhs: &[f64], solution: &[f64], objective: f64) -> Result<()> {
        let observed = normal.multiply(solution);
        let residuals: Vec<f64> = observed.iter().zip(rhs).map(|(a,b)| (a-b)*(a-b)).collect();
        let rhs_squares: Vec<f64> = rhs.iter().map(|v| v*v).collect();
        self.residual_square += pairwise_sum(&residuals);
        self.rhs_square += pairwise_sum(&rhs_squares);
        self.objective += objective;
        self.nonzeros += normal.nonzeros();
        self.unknowns += normal.size;
        ensure!([self.residual_square,self.rhs_square,self.objective].iter().all(|x| x.is_finite()),
            "nonfinite MLPG diagnostics");
        Ok(())
    }
}

// Observation rows have at most two nonzero spatial entries. Original mode
// uses one independent scalar per call. The legacy joint diagnostic uses
// the actual 17-dimensional adjacent-gap component; remaining LSF/gain
// dimensions are independent. No component-specific model substitution.
fn solve_component(means: &[f64], variances: &[f64], frames: usize, observed_dimensions: usize,
    spatial_rows: &[(usize,Vec<(usize,f64)>)], underlying: usize, drop_dynamic: bool,
    diagnostics: &mut Diagnostics) -> Result<Vec<f64>>
{
    let size = frames.checked_mul(underlying).context("MLPG unknown count overflow")?;
    let spatial_span = spatial_rows.iter().map(|(_,row)| {
        row.last().map_or(0, |last| last.0-row[0].0)
    }).max().unwrap_or(0);
    let width = 2*underlying+spatial_span;
    let mut normal = SymmetricBand::new(size,width)?;
    let mut rhs = vec![0.0;size];
    // Match Python's normal-equation accumulation in static/delta/delta2 order.
    for (window_index,window) in WINDOWS.iter().enumerate() {
        for frame in 0..frames {
            if drop_dynamic && window_index != 0 && (frame == 0 || frame+1 == frames) { continue; }
            let (temporal,count) = temporal_terms(frame,frames,window);
            for (dimension,spatial) in spatial_rows {
                let offset = (frame*3+window_index)*observed_dimensions+dimension;
                let precision = 1.0/variances[offset];
                ensure!(precision.is_finite(), "nonfinite inverse MLPG variance");
                let weighted_mean = precision*means[offset];
                ensure!(weighted_mean.is_finite(), "nonfinite weighted MLPG mean");
                let mut terms = [(0usize,0.0);6];
                let mut used = 0;
                for &(time,coefficient) in &temporal[..count] {
                    for &(coordinate,weight) in spatial {
                        terms[used] = (time*underlying+coordinate,coefficient*weight);
                        used += 1;
                    }
                }
                for &(row,value) in &terms[..used] {
                    rhs[row] += value*weighted_mean;
                    ensure!(rhs[row].is_finite(), "nonfinite MLPG right hand side");
                    for &(column,other) in &terms[..used] {
                        if row >= column { normal.add(row,column,value*(other*precision))?; }
                    }
                }
            }
        }
    }
    let solution = normal.solve(&rhs)?;
    let mut objectives = [0.0;3];
    for (window_index,window) in WINDOWS.iter().enumerate() {
        let mut errors = Vec::with_capacity(frames*spatial_rows.len());
        for frame in 0..frames {
            let (temporal,count) = temporal_terms(frame,frames,window);
            for (dimension,spatial) in spatial_rows {
                let offset = (frame*3+window_index)*observed_dimensions+dimension;
                if drop_dynamic && window_index != 0 && (frame == 0 || frame+1 == frames) {
                    errors.push(0.0);
                    continue;
                }
                let mut prediction = 0.0;
                for &(time,coefficient) in &temporal[..count] {
                    for &(coordinate,weight) in spatial {
                        prediction += coefficient*weight*solution[time*underlying+coordinate];
                    }
                }
                let difference = prediction-means[offset];
                errors.push(difference*difference*(1.0/variances[offset]));
            }
        }
        objectives[window_index] = pairwise_sum(&errors);
    }
    diagnostics.collect(&normal,&rhs,&solution,objectives.iter().sum())?;
    Ok(solution)
}

pub fn generate_independent_trajectory(means: &[f64], variances: &[f64], frames: usize,
    dimensions: usize, drop_dynamic: bool) -> Result<(Vec<f64>,Value)>
{
    validate_observations(means,variances,frames,dimensions)?;
    let mut values = vec![0.0;frames*dimensions];
    let mut diagnostics = Diagnostics::default();
    for dimension in 0..dimensions {
        let solution = solve_component(means,variances,frames,dimensions,
            &[(dimension,vec![(0,1.0)])],1,drop_dynamic,&mut diagnostics)?;
        for (frame,&value) in solution.iter().enumerate() { values[frame*dimensions+dimension] = value; }
    }
    Ok((values,diagnostics.report(drop_dynamic)))
}
fn validate_observations(means: &[f64], variances: &[f64], frames: usize, dimensions: usize) -> Result<()> {
    let length = frames.checked_mul(3).and_then(|n| n.checked_mul(dimensions)).context("MLPG shape overflow")?;
    ensure!(frames > 0 && dimensions > 0 && means.len() == length && variances.len() == length,
        "expected equal nonempty [T,3,D] means and variances");
    ensure!(means.iter().all(|v| v.is_finite()) && variances.iter().all(|v| v.is_finite() && *v > 0.0),
        "MLPG requires finite means and strictly positive variances");
    Ok(())
}

fn original_gap_transform(value: f32) -> f64 {
    let x = f64::from(value);
    let transformed = if x > 0.10000000149011612 { x-0.03000000163912775 }
        else if x > 0.05000000074505806 { x*0.699999988079071 }
        else if x > 0.019999999552965164 {
            x*(0.699999988079071+(0.05000000074505806-x)*0.30000001192092896/0.030000001192092896)
        } else { x };
    f64::from(transformed as f32)
}
pub fn original_lsf_correction(observations: &[f64], frames: usize) -> Result<(Vec<f64>,Value)> {
    ensure!(frames > 0 && observations.len() == frames*40 && observations.iter().all(|v| v.is_finite()),
        "expected finite [T,40] generated spectral observations");
    let mut values: Vec<f32> = observations.iter().map(|&v| v as f32).collect();
    ensure!(values.iter().all(|v| v.is_finite()), "generated spectral observations overflow float32 frame storage");
    let mut gaps = vec![[0.0_f64;16];frames];
    let mut original = vec![0.0_f64;frames*24];
    for frame in 0..frames {
        for index in 0..16 { gaps[frame][index] = original_gap_transform(values[frame*40+index]); }
        for index in 0..24 { original[frame*24+index] = f64::from(values[frame*40+16+index]); }
    }
    // Index order is deliberately outermost: each subsequent index reads
    // its already-corrected left neighbor, including float32 rounding.
    for _ in 0..3 {
        for index in 1..16 {
            for frame in 0..frames {
                let left_gap = gaps[frame][index-1];
                let right_gap = gaps[frame][index];
                let denominator = left_gap*left_gap+right_gap*right_gap;
                ensure!(denominator > 0.0, "original LSF correction has two zero adjacent gaps");
                let left = f64::from(values[frame*40+16+index-1]);
                let right = f64::from(values[frame*40+16+index+1]);
                let value = left+(left_gap*left_gap*(right-left)
                    +left_gap*right_gap*(right_gap-left_gap))/denominator;
                ensure!(value.is_finite() && (value as f32).is_finite(), "nonfinite original LSF correction");
                values[frame*40+16+index] = value as f32;
            }
        }
    }
    let mut result = Vec::with_capacity(frames*24);
    let mut maximum = 0.0_f64;
    for frame in 0..frames {
        for index in 0..24 {
            let value = f64::from(values[frame*40+16+index]);
            maximum = maximum.max((value-original[frame*24+index]).abs());
            result.push(value);
        }
    }
    Ok((result,json!({"method":"three in-place forward passes of original adjacent-gap correction",
        "dispatch_rva":"0x4c400","correction_rva":"0x4b6b0","gap_transform_rva":"0x486d0",
        "passes":3,"updated_lsf_indices_0based":(1..16).collect::<Vec<_>>(),
        "frame_storage":"float32, matching original frame records","maximum_change_radians":maximum})))
}

pub fn spectral_mlpg(means: &[f64], variances: &[f64], frames: usize, mode: &str) -> Result<(Vec<f64>,Value)> {
    validate_observations(means,variances,frames,41)?;
    ensure!(matches!(mode,"original"|"joint-reconstruction"), "unknown spectral MLPG mode");
    if mode == "joint-reconstruction" {
        let mut diagnostics = Diagnostics::default();
        let mut rows: Vec<(usize,Vec<(usize,f64)>)> = (0..16).map(|index|
            (index,vec![(index,-1.0),(index+1,1.0)])).collect();
        rows.extend((0..17).map(|index| (16+index,vec![(index,1.0)])));
        let joint = solve_component(means,variances,frames,41,&rows,17,false,&mut diagnostics)?;
        let mut result = vec![0.0;frames*25];
        for frame in 0..frames { result[frame*25..frame*25+17].copy_from_slice(&joint[frame*17..frame*17+17]); }
        for dimension in 17..25 {
            let observed = if dimension == 24 { 40 } else { dimension+16 };
            let solution = solve_component(means,variances,frames,41,&[(observed,vec![(0,1.0)])],1,false,&mut diagnostics)?;
            for frame in 0..frames { result[frame*25+dimension] = solution[frame]; }
        }
        let mut report = diagnostics.report(false);
        report["spectral_generation"] = json!("legacy joint spatial/temporal reconstruction; not original");
        return Ok((result,report));
    }
    let (observations,mut report) = generate_independent_trajectory(means,variances,frames,41,true)?;
    let mut spectral = Vec::with_capacity(frames*40);
    for frame in 0..frames { spectral.extend_from_slice(&observations[frame*41..frame*41+40]); }
    let (lsf,correction) = original_lsf_correction(&spectral,frames)?;
    let mut result = Vec::with_capacity(frames*25);
    for frame in 0..frames {
        result.extend_from_slice(&lsf[frame*24..frame*24+24]);
        result.push(observations[frame*41+40]);
    }
    report["spectral_generation"] = json!("original independent 41-dimensional MLPG followed by local LSF correction");
    report["spectral_dispatch_rva"] = json!("0x5a8c0");
    report["lsf_correction"] = correction;
    report["last_scalar"] = json!("generated log-domain gain; original exponentiation RVA0x5ac21/0x5ac28");
    Ok((result,report))
}

pub fn pitch_mlpg(triplets: &[f64], voiced: &[bool], mode: &str) -> Result<(Vec<f64>,Value)> {
    let frames = voiced.len();
    ensure!(frames > 0 && triplets.len() == frames*9 && triplets.iter().all(|v| v.is_finite()),
        "expected finite [T,3,3] pitch probability/mean/variance triplets");
    ensure!(matches!(mode,"original"|"joint-reconstruction"), "unknown pitch MLPG mode");
    let compact: Vec<usize> = (0..frames).filter(|&frame| mode == "joint-reconstruction" || voiced[frame]).collect();
    let mut result = vec![0.0;frames];
    let mut report = if mode == "original" {
        json!({"pitch_generation":"original compact voiced timeline; original voicing classification supplied separately",
            "voiced_index_builder_rva":"0x50f30","normal_equation_builder_rva":"0x4fe50",
            "voiced_frame_count":compact.len(),"unvoiced_frame_count":frames-compact.len(),
            "unvoiced_log_f0_placeholder":0.0})
    } else { json!({"pitch_generation":"legacy all-frame reconstruction including unvoiced observations"}) };
    if !compact.is_empty() {
        let mut means = Vec::with_capacity(compact.len()*3);
        let mut variances = Vec::with_capacity(compact.len()*3);
        for &frame in &compact {
            for window in 0..3 {
                means.push(triplets[frame*9+window*3+1]);
                variances.push(triplets[frame*9+window*3+2]);
            }
        }
        let (trajectory,solver) = generate_independent_trajectory(&means,&variances,compact.len(),1,mode == "original")?;
        for (&frame,&value) in compact.iter().zip(&trajectory) { result[frame] = value; }
        let object = report.as_object_mut().context("invalid pitch report")?;
        for (key,value) in solver.as_object().context("invalid solver report")? { object.insert(key.clone(),value.clone()); }
    }
    Ok((result,report))
}

pub fn stabilize_lsf(values: &[f64], frames: usize, gap: f64) -> Result<(Vec<f64>,Value)> {
    ensure!(frames > 0 && values.len() == frames*24 && values.iter().all(|v| v.is_finite())
        && gap > 0.0 && gap < std::f64::consts::PI/25.0,
        "expected finite [T,24] LSFs and a valid minimum gap");
    let high = std::f64::consts::PI-gap-23.0*gap;
    let mut result = Vec::with_capacity(values.len());
    let mut changed = 0;
    let mut maximum = 0.0_f64;
    for frame in 0..frames {
        let mut levels = [0.0_f64;24];
        let mut counts = [0usize;24];
        let mut used = 0;
        for index in 0..24 {
            levels[used] = values[frame*24+index]-index as f64*gap;
            counts[used] = 1;
            used += 1;
            while used > 1 && levels[used-2] > levels[used-1] {
                let count = counts[used-2]+counts[used-1];
                levels[used-2] = (levels[used-2]*counts[used-2] as f64+levels[used-1]*counts[used-1] as f64)/count as f64;
                counts[used-2] = count;
                used -= 1;
            }
        }
        let mut index = 0;
        let mut frame_changed = false;
        for block in 0..used {
            for _ in 0..counts[block] {
                let value = levels[block].clamp(gap,high)+index as f64*gap;
                let change = (value-values[frame*24+index]).abs();
                frame_changed |= change > 1e-10;
                maximum = maximum.max(change);
                result.push(value);
                index += 1;
            }
        }
        changed += usize::from(frame_changed);
    }
    Ok((result,json!({"method":"bounded isotonic projection","minimum_gap_radians":gap,
        "changed_frames":changed,"maximum_change_radians":maximum})))
}
