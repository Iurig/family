//! The crossover: splits the DI into bands.
//!
//! It is a chain of Linkwitz-Riley splits, lowest first. Each split takes off one band below its
//! split point and passes the rest up to the next split, so the highest band is what is left above
//! the highest split point. A band split off early never passes through the splits above it, so it
//! goes through an allpass for each of them, matching the phase they give the bands that do. The
//! summed bands are then the input through every split's allpass: the same level at every
//! frequency, only the phase changes, as with any Linkwitz-Riley crossover. The stated
//! reconstruction tolerance is 0 dB ± 0.01 dB from 20 Hz to 20 kHz at 44.1, 48 and 96 kHz; the
//! waveform itself is not rebuilt sample for sample.
//!
//! The chain works for any number of bands from 2 up; band `k` needs an allpass for each split
//! point above its upper edge, so `(BANDS - 1)(BANDS - 2) / 2` allpasses in all.
//!
//! At 12 dB per octave, every split inverts what it passes up, so the bands come out with
//! alternating polarity; that is what makes them sum flat.
//!
//! Filters run in `f64`, so rounding stays far below the reconstruction tolerance at low split
//! points and high sample rates.

use crate::convert::f64_to_f32;

/// How steeply the bands roll off past their split points.
///
/// With the `serde` feature, it is written as `12` or `24` (dB per octave).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize), serde(try_from = "u32"))]
pub enum Steepness {
    /// 12 dB per octave (2nd-order Linkwitz-Riley).
    Db12,
    /// 24 dB per octave (4th-order Linkwitz-Riley).
    #[default]
    Db24,
}

/// A steepness that isn't 12 or 24 dB per octave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedSteepness(pub u32);

impl core::fmt::Display for UnsupportedSteepness {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "steepness must be 12 or 24 dB per octave, not {}",
            self.0
        )
    }
}

impl TryFrom<u32> for Steepness {
    type Error = UnsupportedSteepness;

    fn try_from(db_per_octave: u32) -> Result<Self, Self::Error> {
        match db_per_octave {
            12 => Ok(Self::Db12),
            24 => Ok(Self::Db24),
            other => Err(UnsupportedSteepness(other)),
        }
    }
}

/// Splits a signal into `BANDS` bands, one sample at a time. Band 0 is the lowest.
///
/// Stable Rust can't size an array `BANDS - 1` long, so the arrays below hold `BANDS` entries and
/// the last split, and the allpasses past each band's own, go unused.
#[derive(Debug, Clone)]
pub struct Crossover<const BANDS: usize> {
    /// One split per split point, lowest first: split `k` takes off band `k`. The last is unused.
    splits: [LinkwitzRiley; BANDS],
    /// For each band, an allpass per split point above its upper edge, lowest first: band `k` has
    /// `BANDS - 2 - k` of them (none for the top two bands), and the rest are unused.
    allpasses: [[Biquad; BANDS]; BANDS],
}

impl<const BANDS: usize> Crossover<BANDS> {
    /// Creates a crossover with `BANDS - 1` split points `split_hz` (in Hz, in any order) and
    /// `steepness`, at `sample_rate` (in Hz).
    ///
    /// Each split point is kept between 1 Hz and 45% of the sample rate, a safety bound that keeps
    /// the filters stable. Extra split points are ignored; missing ones are put at the top of that
    /// range, which leaves the bands above them almost empty.
    #[must_use]
    pub fn new(split_hz: &[f32], steepness: Steepness, sample_rate: f64) -> Self {
        const { assert!(BANDS >= 2, "a crossover needs at least two bands") };
        let mut sorted = [f32::INFINITY; BANDS];
        for (point, &hz) in sorted.iter_mut().zip(split_hz.iter().take(BANDS - 1)) {
            *point = hz;
        }
        sorted.sort_unstable_by(f32::total_cmp);
        let k = sorted.map(|hz| prewarp(f64::from(hz), sample_rate));
        Self {
            splits: k.map(|k| LinkwitzRiley::new(k, steepness)),
            allpasses: core::array::from_fn(|band| {
                core::array::from_fn(|i| match k.get(band + 1 + i) {
                    Some(&k) if band + 1 + i < BANDS - 1 => Biquad::allpass(k, steepness),
                    _ => Biquad::IDENTITY,
                })
            }),
        }
    }

    /// Turns one sample into its bands, lowest first.
    pub fn process(&mut self, sample: f32) -> [f32; BANDS] {
        let mut bands = [0.0; BANDS];
        let mut rest = f64::from(sample);
        let below_highest = bands.len() - 1;
        for (index, ((band, split), allpasses)) in bands
            .iter_mut()
            .zip(&mut self.splits)
            .zip(&mut self.allpasses)
            .take(below_highest)
            .enumerate()
        {
            let (low, high) = split.process(rest);
            *band = allpasses
                .iter_mut()
                .take(below_highest - 1 - index)
                .fold(low, |x, allpass| allpass.process(x));
            rest = high;
        }
        if let Some(highest) = bands.last_mut() {
            *highest = rest;
        }
        bands.map(f64_to_f32)
    }
}

/// The bilinear transform's prewarped frequency, `tan(π f / fs)`, with `f` kept between 1 Hz and
/// 45% of `fs`.
#[must_use]
pub fn prewarp(hz: f64, sample_rate: f64) -> f64 {
    let hz = hz.clamp(1.0, 0.45 * sample_rate);
    libm::tan(core::f64::consts::PI * hz / sample_rate)
}

/// One Linkwitz-Riley split into a lowpass and a highpass whose sum is an allpass.
#[derive(Debug, Clone)]
struct LinkwitzRiley {
    lowpass: [Biquad; 2],
    highpass: [Biquad; 2],
    /// The 12 dB split's halves only sum to an allpass with the highpass inverted.
    highpass_sign: f64,
}

impl LinkwitzRiley {
    fn new(k: f64, steepness: Steepness) -> Self {
        match steepness {
            Steepness::Db12 => Self {
                lowpass: [Biquad::lowpass(k, 0.5), Biquad::IDENTITY],
                highpass: [Biquad::highpass(k, 0.5), Biquad::IDENTITY],
                highpass_sign: -1.0,
            },
            Steepness::Db24 => Self {
                lowpass: [Biquad::lowpass(k, BUTTERWORTH_Q); 2],
                highpass: [Biquad::highpass(k, BUTTERWORTH_Q); 2],
                highpass_sign: 1.0,
            },
        }
    }

    /// Returns the low and high halves of one sample.
    fn process(&mut self, sample: f64) -> (f64, f64) {
        let low = self
            .lowpass
            .iter_mut()
            .fold(sample, |x, stage| stage.process(x));
        let high = self
            .highpass
            .iter_mut()
            .fold(sample, |x, stage| stage.process(x));
        (low, self.highpass_sign * high)
    }
}

/// The Q of a 2nd-order Butterworth section, `1/√2`.
pub const BUTTERWORTH_Q: f64 = core::f64::consts::FRAC_1_SQRT_2;

/// A second-order filter section in transposed direct form II, with `a0` normalised to 1.
#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    s1: f64,
    s2: f64,
}

impl Biquad {
    /// Passes the signal through unchanged.
    const IDENTITY: Self = Self::from_coefficients(1.0, 0.0, 0.0, 0.0, 0.0);

    const fn from_coefficients(b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            s1: 0.0,
            s2: 0.0,
        }
    }

    /// The shared denominator of the 2nd-order sections at prewarped frequency `k`: `(a1, a2,
    /// 1/a0)`.
    fn denominator(k: f64, q: f64) -> (f64, f64, f64) {
        let norm = 1.0 / (1.0 + k / q + k * k);
        (
            2.0 * (k * k - 1.0) * norm,
            (1.0 - k / q + k * k) * norm,
            norm,
        )
    }

    /// The bilinear transform of `1 / (s² + s/Q + 1)`.
    fn lowpass(k: f64, q: f64) -> Self {
        let (a1, a2, norm) = Self::denominator(k, q);
        let b0 = k * k * norm;
        Self::from_coefficients(b0, 2.0 * b0, b0, a1, a2)
    }

    /// The bilinear transform of `s² / (s² + s/Q + 1)` at prewarped frequency `k` (see
    /// [`prewarp`]).
    #[must_use]
    pub fn highpass(k: f64, q: f64) -> Self {
        let (a1, a2, norm) = Self::denominator(k, q);
        Self::from_coefficients(norm, -2.0 * norm, norm, a1, a2)
    }

    /// The allpass that a split with this steepness at prewarped frequency `k` sums to.
    fn allpass(k: f64, steepness: Steepness) -> Self {
        match steepness {
            // (1 - s) / (1 + s)
            Steepness::Db12 => {
                let pole = (k - 1.0) / (k + 1.0);
                Self::from_coefficients(pole, 1.0, 0.0, pole, 0.0)
            }
            // (s² - √2 s + 1) / (s² + √2 s + 1)
            Steepness::Db24 => {
                let (a1, a2, _) = Self::denominator(k, BUTTERWORTH_Q);
                Self::from_coefficients(a2, a1, 1.0, a1, a2)
            }
        }
    }

    /// Filters one sample.
    pub fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.s1;
        self.s1 = self.b1 * x + self.s2 - self.a1 * y;
        self.s2 = self.b2 * x - self.a2 * y;
        y
    }
}
