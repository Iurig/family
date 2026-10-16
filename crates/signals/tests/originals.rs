//! Each generator plays exactly what the plugin test it came from played, bit for bit, so a test
//! moving onto this crate doesn't change. The `original` module holds those tests' own copies,
//! verbatim but for their names and where a fixed rate or seed became a parameter.
#![expect(
    clippy::suboptimal_flops,
    reason = "the originals' arithmetic, kept as written"
)]

use family_signals::{
    Xorshift64, bright_note, broadband_note, broadband_partials, click, noise, noise32, note, peak,
    pluck, quick_pluck, rms, sine, sine_samples, slow_click, synthesize,
};

#[expect(
    clippy::unwrap_used,
    reason = "the originals' own unwraps, kept as written; a failure fails the test"
)]
mod original {
    use core::f64::consts::TAU;

    use family_dsp::convert::{bits_to_unit, count_to_f64, f64_to_f32, seconds_to_samples};

    /// Drop's `synthesize` (`crates/dsp/tests/shared/signals.rs`).
    pub fn synthesize(sample_rate: f64, seconds: f64, signal: impl Fn(f64) -> f64) -> Vec<f32> {
        (0..seconds_to_samples(seconds, sample_rate))
            .map(|index| f64_to_f32(signal(count_to_f64(index) / sample_rate)))
            .collect()
    }

    /// Drop's `sine`.
    pub fn sine(sample_rate: f64, seconds: f64, hz: f64, level: f64) -> Vec<f32> {
        synthesize(sample_rate, seconds, |t| level * (TAU * hz * t).sin())
    }

    /// Drop's `click`, and Doubler's in `crates/dsp/tests/doubler.rs`.
    pub fn click(t: f64, at: f64, level: f64) -> f64 {
        const PARTIALS_HZ: [f64; 5] = [2_300.0, 4_700.0, 6_100.0, 8_300.0, 11_300.0];
        if t < at {
            return 0.0;
        }
        let since = t - at;
        let partials: f64 = PARTIALS_HZ.iter().map(|hz| (TAU * hz * since).sin()).sum();
        level * (1.0 - (-since / 0.000_5).exp()) * (-since / 0.003).exp() * partials
    }

    /// Drop's `note`.
    pub fn note(t: f64, at: f64, hz: f64, level: f64) -> f64 {
        if t < at {
            return 0.0;
        }
        let since = t - at;
        let envelope = (since / 0.002).min(1.0) * (-since / 0.8).exp();
        let tone = (TAU * hz * since).sin()
            + 0.5 * (TAU * 2.0 * hz * since).sin()
            + 0.25 * (TAU * 3.0 * hz * since).sin();
        level * envelope * tone
    }

    /// Drop's `pluck`.
    pub fn pluck(t: f64, at: f64) -> f64 {
        click(t, at, 0.02) + note(t, at, 82.407, 0.3)
    }

    /// Drop's `rms`.
    pub fn rms(signal: &[f32]) -> f64 {
        let sum: f64 = signal.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
        (sum / count_to_f64(signal.len().max(1))).sqrt()
    }

    /// Drop's `peak`.
    pub fn peak(signal: &[f32]) -> f32 {
        signal.iter().fold(0.0, |peak, &s| peak.max(s.abs()))
    }

    /// Drop's `Spacing` (`crates/dsp/tests/shared/mod.rs`).
    pub struct Spacing(pub u64);

    impl Spacing {
        pub fn next(&mut self, low: f64, high: f64) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            low + (high - low) * bits_to_unit(self.0)
        }
    }

    /// Doubler's `synthesize` in `crates/dsp/tests/attack_start.rs` and `onset.rs`.
    pub fn synthesize_at(
        sample_rate: u32,
        seconds: f64,
        mut signal: impl FnMut(f64) -> f64,
    ) -> Vec<f32> {
        let count = seconds_to_samples(seconds, f64::from(sample_rate));
        (0..count)
            .map(|index| {
                f64_to_f32(signal(family_dsp::convert::sample_to_seconds(
                    index,
                    sample_rate,
                )))
            })
            .collect()
    }

    /// Doubler's `note` in `crates/dsp/tests/attack_start.rs`.
    pub fn bright_note(t: f64, at: f64, level: f64, decay: f64) -> f64 {
        if t < at {
            return 0.0;
        }
        let since = t - at;
        let harmonics: f64 = (1..=24)
            .map(|harmonic| {
                let harmonic = f64::from(harmonic);
                (-since * harmonic / decay).exp() * (TAU * 82.4 * harmonic * since).sin() / harmonic
            })
            .sum();
        level * (1.0 - (-since / 0.001).exp()) * harmonics / 2.0
    }

    /// Doubler's `click` in `crates/dsp/tests/attack_start.rs`, and `pick` in `onset.rs`.
    pub fn slow_click(t: f64, at: f64, level: f64) -> f64 {
        const PARTIALS_HZ: [f64; 6] = [1_900.0, 3_100.0, 4_700.0, 6_100.0, 8_300.0, 11_300.0];
        if t < at {
            return 0.0;
        }
        let since = t - at;
        let click: f64 = PARTIALS_HZ.iter().map(|hz| (TAU * hz * since).sin()).sum();
        level * (1.0 - (-since / 0.001).exp()) * (-since / 0.003).exp() * click / 2.0
    }

    /// Doubler's `pluck` in `crates/dsp/tests/doubler.rs`.
    pub fn quick_pluck(t: f64, at: f64) -> f64 {
        if t < at {
            return 0.0;
        }
        let since = t - at;
        let note = 0.3
            * (-since / 0.4).exp()
            * ((TAU * 82.4 * since).sin() + 0.5 * (TAU * 164.8 * since).sin());
        click(t, at, 0.02) + note
    }

    /// Doubler's `plucks` in `crates/dsp/tests/doubler.rs`.
    pub fn quick_plucks(sample_rate: f64, seconds: f64, at: &[f64]) -> Vec<f32> {
        let count = seconds_to_samples(seconds, sample_rate);
        (0..count)
            .map(|index| {
                let t = count_to_f64(index) / sample_rate;
                f64_to_f32(at.iter().map(|&at| quick_pluck(t, at)).sum())
            })
            .collect()
    }

    /// Doubler's `sine` in `crates/dsp/tests/analysis.rs`, with its fixed rate as a parameter.
    pub fn sine_samples(
        sample_rate: f64,
        frequency: f64,
        amplitude: f64,
        length: usize,
    ) -> impl Iterator<Item = f32> {
        (0..length).map(move |n| {
            let phase = core::f64::consts::TAU * frequency * f64::from(u32::try_from(n).unwrap())
                / sample_rate;
            f64_to_f32(amplitude * phase.sin())
        })
    }

    /// Doubler's `partials` in `crates/dsp/tests/timing.rs`.
    pub fn broadband_partials() -> Vec<(f64, f64)> {
        const COUNT: u32 = 120;
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        (0..COUNT)
            .map(|n| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let phase = f64::from(u32::try_from(state >> 40).unwrap_or(0))
                    / f64::from(1_u32 << 24)
                    * TAU;
                (
                    50.0 * (8_000.0_f64 / 50.0).powf(f64::from(n) / f64::from(COUNT - 1)),
                    phase,
                )
            })
            .collect()
    }

    /// Doubler's `note` in `crates/dsp/tests/timing.rs`.
    pub fn broadband_note(partials: &[(f64, f64)], at: f64, t: f64) -> f64 {
        if t < at {
            return 0.0;
        }
        let tone: f64 = partials
            .iter()
            .map(|&(hz, phase)| (TAU * hz * t + phase).sin())
            .sum();
        0.3 * (-(t - at) / 0.4).exp() * tone
            / f64::from(u32::try_from(partials.len()).unwrap()).sqrt()
    }

    /// Doubler's `noise` in `crates/dsp/tests/correlation.rs`.
    pub fn noise(seed: u64) -> impl Iterator<Item = f32> {
        let mut state = seed | 1;
        core::iter::repeat_with(move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let top_24_bits = u32::try_from(state >> 40).unwrap();
            f32::from(u16::try_from(top_24_bits >> 8).unwrap()) / 32_768.0 - 1.0
        })
    }

    /// Doubler's `noise` in `crates/dsp/tests/delay.rs`.
    pub fn noise32(len: usize) -> Vec<f32> {
        let mut state: u32 = 0x1234_5678;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let [high, low, ..] = state.to_be_bytes();
                f32::from(u16::from_be_bytes([high, low])) / 32_768.0 - 1.0
            })
            .collect()
    }
}

/// The sample rates the plugins test at.
const SAMPLE_RATES: [u32; 3] = [44_100, 48_000, 96_000];

/// The bits of each sample, so a NaN or a signed zero would compare too.
fn bits(signal: &[f32]) -> Vec<u32> {
    signal.iter().map(|s| s.to_bits()).collect()
}

/// A signal of time.
type Signal<'a> = &'a dyn Fn(f64) -> f64;

/// Attack times that fall at different points of a sample at every rate.
const AT: [f64; 3] = [0.0, 0.010_3, 0.123_45];

#[test]
fn synthesize_samples_as_drops_and_doublers_did() {
    for sample_rate in SAMPLE_RATES {
        let rate = f64::from(sample_rate);
        let signal = |t: f64| (t * 1_234.5).sin() + t;
        let ours = synthesize(rate, 0.2, signal);

        assert_eq!(bits(&ours), bits(&original::synthesize(rate, 0.2, signal)));
        assert_eq!(
            bits(&ours),
            bits(&original::synthesize_at(sample_rate, 0.2, signal))
        );
    }
}

#[test]
fn the_sines_match_drops_and_doublers() {
    for sample_rate in SAMPLE_RATES.map(f64::from) {
        for hz in [55.0, 80.0, 440.0, 1_000.0] {
            assert_eq!(
                bits(&sine(sample_rate, 0.5, hz, 0.5)),
                bits(&original::sine(sample_rate, 0.5, hz, 0.5))
            );
            let theirs: Vec<f32> = original::sine_samples(sample_rate, hz, 0.5, 24_000).collect();
            assert_eq!(
                bits(&sine_samples(sample_rate, 24_000, hz, 0.5)),
                bits(&theirs)
            );
        }
    }
}

#[test]
fn the_clicks_notes_and_plucks_match_drops_and_doublers() {
    for sample_rate in SAMPLE_RATES {
        let rate = f64::from(sample_rate);
        for at in AT {
            let pairs: [(Signal, Signal); 7] = [
                (&|t| click(t, at, 0.1), &|t| original::click(t, at, 0.1)),
                (&|t| slow_click(t, at, 0.2), &|t| {
                    original::slow_click(t, at, 0.2)
                }),
                (&|t| note(t, at, 55.0, 0.25), &|t| {
                    original::note(t, at, 55.0, 0.25)
                }),
                (&|t| bright_note(t, at, 0.5, 1.0), &|t| {
                    original::bright_note(t, at, 0.5, 1.0)
                }),
                (&|t| pluck(t, at), &|t| original::pluck(t, at)),
                (&|t| quick_pluck(t, at), &|t| original::quick_pluck(t, at)),
                (
                    &|t| bright_note(t, at, 0.8, 2.0) + slow_click(t, at, 0.3),
                    &|t| original::bright_note(t, at, 0.8, 2.0) + original::slow_click(t, at, 0.3),
                ),
            ];
            for (index, (ours, theirs)) in pairs.iter().enumerate() {
                assert_eq!(
                    bits(&synthesize(rate, 0.3, ours)),
                    bits(&original::synthesize_at(sample_rate, 0.3, theirs)),
                    "signal {index} at {at} s, {sample_rate} Hz"
                );
            }
        }
    }
}

#[test]
fn summed_quick_plucks_match_doublers_plucks() {
    let at: Vec<f64> = (0..12).map(|n| 0.1 + 0.1 * f64::from(n)).collect();
    for sample_rate in SAMPLE_RATES.map(f64::from) {
        let ours = synthesize(sample_rate, 1.5, |t| {
            at.iter().map(|&at| quick_pluck(t, at)).sum()
        });

        assert_eq!(
            bits(&ours),
            bits(&original::quick_plucks(sample_rate, 1.5, &at))
        );
    }
}

#[test]
fn the_broadband_note_matches_doublers() {
    let partials = broadband_partials(120);
    let theirs = original::broadband_partials();
    let to_bits = |partials: &[(f64, f64)]| -> Vec<(u64, u64)> {
        partials
            .iter()
            .map(|&(hz, phase)| (hz.to_bits(), phase.to_bits()))
            .collect()
    };
    assert_eq!(to_bits(&partials), to_bits(&theirs));

    let sample_rate = 48_000;
    let at = [0.1, 0.6, 1.1];
    assert_eq!(
        bits(&synthesize(f64::from(sample_rate), 1.6, |t| {
            at.iter().map(|&at| broadband_note(t, at, &partials)).sum()
        })),
        bits(&original::synthesize_at(sample_rate, 1.6, |t| {
            at.iter()
                .map(|&at| original::broadband_note(&theirs, at, t))
                .sum()
        }))
    );
}

#[test]
fn the_noises_match_doublers() {
    for seed in [0, 1, 3, 9, 10, 0x9e37_79b9_7f4a_7c15] {
        let ours: Vec<f32> = noise(seed).take(10_000).collect();
        let theirs: Vec<f32> = original::noise(seed).take(10_000).collect();
        assert_eq!(bits(&ours), bits(&theirs), "seed {seed}");
    }

    let ours: Vec<f32> = noise32(0x1234_5678).take(20_000).collect();
    assert_eq!(bits(&ours), bits(&original::noise32(20_000)));
}

#[test]
fn the_generator_matches_drops_spacing() {
    let seed = 0x9e37_79b9_7f4a_7c15;
    let mut ours = Xorshift64::new(seed);
    let mut theirs = original::Spacing(seed);

    for _ in 0..1_000 {
        assert_eq!(
            ours.uniform(0.12, 0.16).to_bits(),
            theirs.next(0.12, 0.16).to_bits()
        );
    }
}

#[test]
fn the_levels_match_drops() {
    let signal = synthesize(48_000.0, 0.5, |t| pluck(t, 0.01) + pluck(t, 0.23));

    assert_eq!(rms(&signal).to_bits(), original::rms(&signal).to_bits());
    assert_eq!(peak(&signal).to_bits(), original::peak(&signal).to_bits());
    assert_eq!(rms(&[]).to_bits(), original::rms(&[]).to_bits());
    assert_eq!(peak(&[]).to_bits(), original::peak(&[]).to_bits());
}
