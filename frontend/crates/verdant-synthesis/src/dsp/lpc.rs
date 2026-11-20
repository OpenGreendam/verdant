//! Even-order all-pole conversion and one continuous synthesis lattice.
//!
//! The forward conversion follows verdant_acoustics/lpc.py convolution order.
//! Schur admission is strict: neither roots nor reflection coefficients are
//! clipped. Polynomial roots are solved and polished; failure is an error.

use anyhow::{bail, ensure, Result};
use num_complex::Complex64;
use std::f64::consts::PI;

fn convolve_small(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; a.len() + b.len() - 1];
    // np.convolve accumulates its inner dot product in reverse kernel order.
    for (index, value) in out.iter_mut().enumerate() {
        let lower = index.saturating_sub(b.len() - 1);
        let upper = index.min(a.len() - 1);
        for i in lower..=upper { *value += a[i] * b[index - i]; }
    }
    out
}

pub fn lsf_to_lpc(omega: &[f64]) -> Result<Vec<f64>> {
    ensure!(omega.len() >= 2 && omega.len() % 2 == 0 && omega.iter().all(|x| x.is_finite())
        && omega[0] > 0.0 && omega[omega.len() - 1] < PI
        && omega.windows(2).all(|w| w[0] < w[1]),
        "even-order strictly increasing LSFs in (0, pi) required");
    let mut p = vec![1.0];
    let mut q = vec![1.0];
    for &frequency in omega.iter().step_by(2) {
        p = convolve_small(&p, &[1.0, -2.0 * frequency.cos(), 1.0]);
    }
    for &frequency in omega.iter().skip(1).step_by(2) {
        q = convolve_small(&q, &[1.0, -2.0 * frequency.cos(), 1.0]);
    }
    let plus = convolve_small(&p, &[1.0, 1.0]);
    let minus = convolve_small(&q, &[1.0, -1.0]);
    let mut a: Vec<f64> = plus.iter().zip(&minus).map(|(p, q)| 0.5 * (p + q)).collect();
    ensure!(a.iter().all(|v| v.is_finite()) && a.last().unwrap().abs() <= 1e-7,
        "LSF polynomial cancellation failed");
    a.pop();
    let leading = a[0];
    for value in &mut a { *value /= leading; }
    Ok(a)
}

pub fn lpc_to_reflection(coefficients: &[f64]) -> Result<Vec<f64>> {
    ensure!(coefficients.len() >= 2 && coefficients.iter().all(|x| x.is_finite())
        && coefficients[0] != 0.0, "finite nonzero-leading LPC coefficients required");
    let mut a: Vec<f64> = coefficients.iter().map(|v| v / coefficients[0]).collect();
    let order = a.len() - 1;
    let mut reflection = vec![0.0; order];
    let mut previous = vec![0.0; order + 1];
    for m in (1..=order).rev() {
        let k = a[m];
        ensure!(k.is_finite() && k.abs() < 1.0,
            "LPC step-down produced a non-Schur-stable reflection coefficient");
        reflection[m - 1] = k;
        previous[0] = 1.0;
        for i in 1..m { previous[i] = (a[i] - k * a[m-i]) / (1.0 - k * k); }
        a[..m].copy_from_slice(&previous[..m]);
    }
    Ok(reflection)
}

fn evaluate_polynomial(a: &[Complex64], x: Complex64) -> (Complex64, Complex64, Complex64, f64) {
    let mut b = a[0];
    let mut derivative = Complex64::new(0.0, 0.0);
    let mut half_second = derivative;
    let mut bound = b.norm();
    let magnitude = x.norm();
    for &coefficient in &a[1..] {
        half_second = x * half_second + derivative;
        derivative = x * derivative + b;
        b = x * b + coefficient;
        bound = b.norm() + magnitude * bound;
    }
    (b, derivative, half_second, bound)
}

fn laguerre(a: &[Complex64], mut x: Complex64) -> Result<Complex64> {
    let degree = (a.len() - 1) as f64;
    // Bounded fractional steps prevent a symmetric non-real pair from cycling
    // when started on the real axis; every accepted root has a residual bound.
    const FRACTIONS: [f64; 8] = [0.5, 0.25, 0.75, 0.13, 0.38, 0.62, 0.88, 1.0];
    for iteration in 1..=240 {
        let (value, derivative, half_second, bound) = evaluate_polynomial(a, x);
        if value.norm() <= 8.0 * f64::EPSILON * bound { return Ok(x); }
        let g = derivative / value;
        let h = g * g - 2.0 * half_second / value;
        let square = ((degree - 1.0) * (degree * h - g * g)).sqrt();
        let mut denominator = g + square;
        if denominator.norm() < (g - square).norm() { denominator = g - square; }
        let step = if denominator.norm() > 0.0 {
            degree / denominator
        } else {
            Complex64::from_polar(1.0 + x.norm(), iteration as f64)
        };
        let next = x - step;
        if next == x { return Ok(x); }
        x = if iteration % 10 == 0 {
            x - FRACTIONS[((iteration / 10) - 1) % FRACTIONS.len()] * step
        } else { next };
        ensure!(x.re.is_finite() && x.im.is_finite(), "nonfinite polynomial root iteration");
    }
    bail!("polynomial root iteration failed to converge")
}

/// Roots of a descending real polynomial, with exact zero roots retained.
/// Deflation finds starts; each nonzero root is polished on the original
/// polynomial and must satisfy a scale-relative residual before admission.
pub fn polynomial_roots(coefficients: &[f64]) -> Result<Vec<Complex64>> {
    ensure!(coefficients.len() >= 2 && coefficients.iter().all(|v| v.is_finite())
        && coefficients[0] != 0.0, "finite nonconstant polynomial required");
    let mut last = coefficients.len();
    while last > 1 && coefficients[last-1] == 0.0 { last -= 1; }
    let zero_count = coefficients.len() - last;
    let original: Vec<Complex64> = coefficients[..last].iter()
        .map(|&v| Complex64::new(v / coefficients[0], 0.0)).collect();
    let mut current = original.clone();
    let mut roots = Vec::with_capacity(coefficients.len() - 1);
    while current.len() > 1 {
        let root = laguerre(&current, Complex64::new(0.0, 0.0))?;
        roots.push(root);
        let mut previous = current[0];
        for coefficient in current.iter_mut().skip(1) {
            let next = *coefficient + root * previous;
            *coefficient = previous;
            previous = next;
        }
        // Synthetic division above keeps q0 at old a1: shift q back once.
        current.rotate_left(1);
        current.pop();
    }
    for root in &mut roots {
        *root = laguerre(&original, *root)?;
        let (residual, _, _, bound) = evaluate_polynomial(&original, *root);
        ensure!(root.re.is_finite() && root.im.is_finite()
            && residual.norm() <= 256.0 * f64::EPSILON * bound,
            "polynomial root residual exceeds numerical admission bound");
    }
    roots.extend(std::iter::repeat(Complex64::new(0.0, 0.0)).take(zero_count));
    Ok(roots)
}

pub fn maximum_pole_radius(coefficients: &[f64]) -> Result<f64> {
    Ok(polynomial_roots(coefficients)?.iter().map(|r| r.norm()).fold(0.0, f64::max))
}

pub fn lpc_to_lsf(coefficients: &[f64]) -> Result<Vec<f64>> {
    let order = coefficients.len().saturating_sub(1);
    ensure!(order >= 2 && order % 2 == 0 && coefficients.iter().all(|v| v.is_finite())
        && coefficients[0] == 1.0, "finite monic even-order LPC coefficients required");
    lpc_to_reflection(coefficients)?;
    ensure!(maximum_pole_radius(coefficients)? < 1.0, "LPC is not Schur stable");
    let mut extended = coefficients.to_vec();
    extended.push(0.0);
    let mut groups = [Vec::with_capacity(order/2), Vec::with_capacity(order/2)];
    for (group, sign) in groups.iter_mut().zip([1.0, -1.0]) {
        let polynomial: Vec<f64> = extended.iter().zip(extended.iter().rev())
            .map(|(&a, &b)| a + sign * b).collect();
        for root in polynomial_roots(&polynomial)? {
            ensure!((root.norm() - 1.0).abs() <= 1e-5, "LSF roots left the unit circle");
            let phase = root.arg();
            if phase > 1e-8 && phase < PI - 1e-8 { group.push(phase); }
        }
        group.sort_by(f64::total_cmp);
        ensure!(group.len() == order / 2, "incorrect number of LSF roots");
    }
    let result: Vec<f64> = (0..order/2).flat_map(|i| [groups[0][i], groups[1][i]]).collect();
    ensure!(result.windows(2).all(|w| w[0] < w[1]), "LSF roots do not interlace");
    Ok(result)
}

/// All-pole synthesis with exactly one state owner for a complete run.
/// Frame-center PARCOR interpolation is calculated on demand: no samples×order
/// coefficient matrix is allocated, and internal phone boundaries never reset.
pub fn lattice_filter(excitation: &[f64], reflection: &[f64], frames: usize,
    order: usize, hop: usize) -> Result<Vec<f64>> {
    ensure!(frames > 0 && order > 0 && hop > 0 && reflection.len() == frames * order
        && frames.checked_mul(hop) == Some(excitation.len())
        && reflection.iter().all(|v| v.is_finite() && v.abs() < 1.0)
        && excitation.iter().all(|v| v.is_finite()),
        "lattice requires matching duration and stable reflection coefficients");
    let mut backward = vec![0.0; order];
    let mut current = vec![0.0; order];
    let mut output = Vec::with_capacity(excitation.len());
    for (n, &value) in excitation.iter().enumerate() {
        let coordinate = (n as f64 / hop as f64 - 0.5).clamp(0.0, (frames-1) as f64);
        let left = coordinate.floor() as usize;
        let right = (left+1).min(frames-1);
        let fraction = coordinate - left as f64;
        let mut forward = value;
        for m in (0..order).rev() {
            let coefficient = reflection[left*order+m] * (1.0-fraction)
                + reflection[right*order+m] * fraction;
            ensure!(coefficient.abs() < 1.0, "unstable interpolated reflection coefficient");
            forward -= coefficient * backward[m];
            if m+1 < order { current[m+1] = backward[m] + coefficient * forward; }
        }
        current[0] = forward;
        ensure!(forward.is_finite(), "nonfinite lattice waveform");
        output.push(forward);
        std::mem::swap(&mut backward, &mut current);
    }
    Ok(output)
}
