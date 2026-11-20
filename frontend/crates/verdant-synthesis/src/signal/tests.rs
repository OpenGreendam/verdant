//! Small independent numerical contracts; the parent runs these after integration.
use super::*;

fn close(actual: &[f64], expected: &[f64], tolerance: f64) {
    assert_eq!(actual.len(), expected.len());
    for (index, (a,b)) in actual.iter().zip(expected).enumerate() { assert!((a-b).abs() <= tolerance, "sample {index}: {a} != {b}"); }
}

#[test]
fn explicit_polyphase_taps_keep_scipy_center_and_zero_edges() {
    // These are exact upfirdn sums, independent of FFT/FIR design.
    assert_eq!(resample_poly_with_taps(&[1.,2.,3.],2,1,&[1.]).unwrap(), [2.,0.,4.,0.,6.,0.]);
    assert_eq!(resample_poly_with_taps(&[1.,2.,3.,4.,5.],1,2,&[0.25,0.5,0.25]).unwrap(), [1.,3.,3.5]);
    assert_eq!(resample_poly_with_taps(&[1.,2.,3.],6,3,&[1.]).unwrap(), [2.,0.,4.,0.,6.,0.]);
    assert_eq!(resample_poly(&[0.25,-0.25],160,160,5.).unwrap(), [0.25,-0.25]);
    assert_eq!(resample_poly(&vec![0.;80],441,160,5.).unwrap(), vec![0.;221]);
    assert!(resample_poly(&[1.],0,1,5.).is_err());
}

#[test]
fn real_fft_odd_and_even_roundtrip_with_known_dc() {
    for n in [63,64] {
        let source: Vec<f64> = (0..n).map(|i| 0.2 + (2.*PI*3.*i as f64/n as f64).sin()).collect();
        let spectrum = rfft(&source,n).unwrap();
        assert!((spectrum[0].re - 0.2*n as f64).abs() < 1e-12);
        assert!((spectrum[3].im + n as f64/2.).abs() < 1e-12);
        close(&irfft(&spectrum,n).unwrap(),&source,1e-12);
    }
}

#[test]
fn centered_convolution_and_reflection_are_not_causal_or_periodic() {
    close(&fftconvolve_same(&[1.,2.,3.],&[1.,-1.]).unwrap(),&[1.,1.,1.],1e-13);
    close(&fftconvolve_same(&[1.,2.,3.],&[0.25,0.5,0.25]).unwrap(),&[1.,2.,2.],1e-13);
    assert_eq!(maximum_filter1d(&[1.,2.,3.],3,BorderMode::Nearest).unwrap(),[2.,3.,3.]);
    assert_eq!(minimum_filter1d(&[1.,2.,3.],3,BorderMode::Nearest).unwrap(),[1.,1.,2.]);
    close(&uniform_filter1d(&[1.,2.,3.],3,BorderMode::Reflect).unwrap(),&[4./3.,2.,8./3.],1e-15);
    assert_eq!(maximum_filter1d(&[1.,2.,3.],2,BorderMode::Nearest).unwrap(),[1.,2.,3.]);
}

#[test]
fn whole_run_hann_grid_has_no_spectral_batch_reset() {
    let count = 2*256*240+137;
    let source: Vec<f64> = (0..count).map(|i| 0.1*(2.*PI*731.*i as f64/48000.).sin()).collect();
    let spectrum = stft_hann(&source,1024,240,StftBoundary::Zeros,true).unwrap();
    assert_eq!(spectrum.frames,(count+239)/240+1);
    assert_eq!(spectrum.bins,513);
    close(&istft_hann(&spectrum,count,true).unwrap(),&source,1e-12);
}

#[test]
fn pcm16_conversion_uses_even_ties_and_distinct_encode_decode_scales() {
    let ties: Vec<f64> = [-2.5,-1.5,-0.5,0.5,1.5,2.5].into_iter().map(round_ties_even).collect();
    assert_eq!(ties,[-2.,-2.,0.,0.,2.,2.]);
    let codes = pcm16_samples(&[-1.,-0.5,0.,0.5,1.],1.,0.999).unwrap();
    let expected: Vec<f64> = [-32734.,-16384.,0.,16384.,32734.].iter().map(|v| v/32768.).collect();
    assert_eq!(codes,expected);
}

#[test]
fn fixed_increment_headroom_preserves_hot_base_and_both_polarities() {
    let base: Vec<f64> = (0..4800).map(|i| 0.55*(2.*PI*300.*i as f64/48000.).sin()).collect();
    let increment: Vec<f64> = (0..4800).map(|i| (1.+0.8*(2.*PI*7.*i as f64/48000.).sin())*(2.*PI*10500.*i as f64/48000.+0.7).sin()).collect();
    for sign in [-1.,1.] {
        let delta: Vec<f64> = increment.iter().map(|v| sign*v).collect();
        let (result,report) = static_increment_headroom(&base,&delta,0.95).unwrap();
        let gain = report["increment_gain"].as_f64().unwrap();
        assert!(gain > 0. && gain < 1.);
        assert!(peak(&resample_poly(&result,4,1,8.6).unwrap()) <= 0.95+2e-12);
        for (index,value) in result.iter().enumerate() { assert_eq!(*value,base[index]+gain*delta[index]); }
        assert_eq!(report["native_gain"],1.);
        assert_eq!(report["time_varying_gain"],false);
    }
    let hot: Vec<f64> = base.iter().map(|v| v*2.).collect();
    let (result,report) = static_increment_headroom(&hot,&increment,0.95).unwrap();
    assert_eq!(result,hot);
    assert_eq!(report["increment_gain"],0.);
    assert!(static_increment_headroom(&[0.],&[f64::NAN],0.95).is_err());
}
