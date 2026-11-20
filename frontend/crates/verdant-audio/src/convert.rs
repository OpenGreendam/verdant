//! Saturating device conversions. NaN is silence; infinities and finite
//! out-of-range samples saturate. Unsigned silence is the midpoint.
//!
//! cpal/dasp float conversions assume `-1 <= sample < 1`; passing exactly
//! +1 to their packed 24-bit conversions can overflow. Do the scaling here
//! instead, including both full-scale endpoints.

#[inline]
pub fn clean(v: f32) -> f32 {
    if v.is_nan() { 0.0 } else { v.clamp(-1.0, 1.0) }
}

pub trait DeviceSample: Copy {
    fn from_f32(value: f32) -> Self;
}

macro_rules! signed_samples {
    ($($t:ty),+ $(,)?) => {$ (
        impl DeviceSample for $t {
            #[inline]
            fn from_f32(value: f32) -> Self {
                // Float-to-int casts saturate, including the positive endpoint.
                (clean(value) as f64 * -(<$t>::MIN as f64)) as $t
            }
        }
    )+};
}

macro_rules! unsigned_samples {
    ($($t:ty),+ $(,)?) => {$ (
        impl DeviceSample for $t {
            #[inline]
            fn from_f32(value: f32) -> Self {
                let midpoint = (1u64 << (<$t>::BITS - 1)) as f64;
                ((clean(value) as f64 + 1.0) * midpoint) as $t
            }
        }
    )+};
}

signed_samples!(i8, i16, i32, i64);
unsigned_samples!(u8, u16, u32, u64);

#[cfg(feature = "audio-device")]
impl DeviceSample for cpal::I24 {
    #[inline]
    fn from_f32(value: f32) -> Self {
        let n = (clean(value) as f64 * 8_388_608.0) as i32;
        cpal::I24::new_unchecked(n.clamp(-8_388_608, 8_388_607))
    }
}

#[cfg(feature = "audio-device")]
impl DeviceSample for cpal::U24 {
    #[inline]
    fn from_f32(value: f32) -> Self {
        let n = ((clean(value) as f64 + 1.0) * 8_388_608.0) as u32;
        cpal::U24::new_unchecked(n.min(16_777_215) as i32)
    }
}

impl DeviceSample for f32 {
    #[inline]
    fn from_f32(value: f32) -> Self {
        clean(value)
    }
}

impl DeviceSample for f64 {
    #[inline]
    fn from_f32(value: f32) -> Self {
        clean(value) as f64
    }
}

#[inline]
pub fn to<T: DeviceSample>(value: f32) -> T {
    T::from_f32(value)
}

/// Stereo maps to the first two device channels; extra channels are silent.
/// Mono is the arithmetic mean of left and right. Every slot is overwritten,
/// including a trailing partial frame supplied by a malformed backend buffer.
pub fn interleave<T: DeviceSample>(stereo: &[f32], out: &mut [T], channels: usize) {
    if channels == 0 {
        out.fill(T::from_f32(0.0));
        return;
    }
    let converted = (stereo.len() / 2).min(out.len() / channels);
    for (source, dest) in stereo.chunks_exact(2).zip(out.chunks_exact_mut(channels)) {
        if channels == 1 {
            dest[0] = T::from_f32((source[0] + source[1]) * 0.5);
        } else {
            dest[0] = T::from_f32(source[0]);
            dest[1] = T::from_f32(source[1]);
            dest[2..].fill(T::from_f32(0.0));
        }
    }
    out[converted * channels..].fill(T::from_f32(0.0));
}
