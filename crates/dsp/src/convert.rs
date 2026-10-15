//! The audited float↔integer conversions. Every lossy `as` cast in the workspace goes through here.

/// The float value of one step at full scale: `2^(bits - 1)`.
///
/// `bits` must be in `1..=32`.
#[expect(
    clippy::cast_precision_loss,
    reason = "a power of two up to 2^31 is exact in f32"
)]
const fn full_scale(bits: u32) -> f32 {
    (1_u32 << (bits - 1)) as f32
}

/// Converts a signed integer PCM sample with `bits` bits (`1..=32`) to a float in `-1.0..1.0`.
///
/// Exact for up to 24 bits; 32-bit samples lose their lowest bits.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "exact for up to 24 bits, which covers our recordings"
)]
pub fn pcm_to_f32(sample: i32, bits: u32) -> f32 {
    sample as f32 / full_scale(bits)
}

/// Converts a float to a signed integer PCM sample with `bits` bits (`1..=32`),
/// rounding to the nearest step and clamping to the representable range.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is rounded and clamped to the i32 range first"
)]
pub fn f32_to_pcm(value: f32, bits: u32) -> i32 {
    let scale = full_scale(bits);
    libm::roundf(value * scale).clamp(-scale, scale - 1.0) as i32
}

/// Converts a sample index to its time in seconds at `sample_rate`.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "exact below 2^53 samples, about 6000 years at 48 kHz"
)]
pub fn sample_to_seconds(sample: usize, sample_rate: u32) -> f64 {
    sample as f64 / f64::from(sample_rate)
}

/// Converts a count (of samples, say) to `f64`.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "exact below 2^53, far beyond any count of samples we handle"
)]
pub const fn count_to_f64(count: usize) -> f64 {
    count as f64
}

/// Rounds a value down to a whole count. A negative or NaN value is 0; one too large for `usize` is
/// `usize::MAX`.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "rounded down and clamped to 0 first; `as` saturates"
)]
pub fn floor_to_count(value: f64) -> usize {
    libm::floor(value).max(0.0) as usize
}

/// Converts a duration in seconds to the nearest whole number of samples at `sample_rate`.
/// A negative or NaN duration is 0 samples; one too long for `usize` is `usize::MAX`.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "rounded and clamped to 0 first; `as` saturates"
)]
pub fn seconds_to_samples(seconds: f64, sample_rate: f64) -> usize {
    libm::round(seconds * sample_rate).max(0.0) as usize
}

/// Maps 64 random bits to a value spread evenly over `0.0..1.0`, using the top 53.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "a 53-bit integer converts to f64 exactly"
)]
pub const fn bits_to_unit(bits: u64) -> f64 {
    (bits >> 11) as f64 / (1_u64 << 53) as f64
}

/// Converts a time in milliseconds to the nearest whole number of samples at `sample_rate`.
/// A negative or NaN time is 0 samples; one too long for `usize` is `usize::MAX`.
#[must_use]
pub fn ms_to_samples(ms: f32, sample_rate: f64) -> usize {
    seconds_to_samples(f64::from(ms) / 1_000.0, sample_rate)
}

/// Narrows an `f64` to the nearest `f32`. Values beyond the `f32` range become infinite.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "rounding to the nearest f32 is the point"
)]
pub const fn f64_to_f32(value: f64) -> f32 {
    value as f32
}
