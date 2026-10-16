//! What each generator plays: its timing, its levels and its range.

use family_signals::{
    Xorshift64, bright_note, broadband_note, broadband_partials, click, noise, noise32, note, peak,
    pluck, quick_pluck, rms, sine, sine_samples, slow_click, synthesize,
};

const SAMPLE_RATE: f64 = 48_000.0;

#[test]
fn synthesize_samples_the_signal_at_each_samples_time() {
    let signal = synthesize(4.0, 1.0, |t| t);

    assert_eq!(signal, [0.0, 0.25, 0.5, 0.75]);
}

#[test]
fn synthesize_gives_the_nearest_whole_number_of_samples() {
    assert_eq!(synthesize(SAMPLE_RATE, 0.5, |_| 0.0).len(), 24_000);
    assert_eq!(synthesize(10.0, 0.26, |_| 0.0).len(), 3);
    assert_eq!(synthesize(SAMPLE_RATE, 0.0, |_| 1.0), [] as [f32; 0]);
}

#[test]
fn a_sine_has_its_level_as_peak_and_its_level_over_root_two_as_rms() {
    for signal in [
        sine(SAMPLE_RATE, 1.0, 100.0, 0.5),
        sine_samples(SAMPLE_RATE, 48_000, 100.0, 0.5),
    ] {
        assert!((peak(&signal) - 0.5).abs() < 1e-6, "{}", peak(&signal));
        assert!(
            (rms(&signal) - 0.5 / 2_f64.sqrt()).abs() < 1e-6,
            "{}",
            rms(&signal)
        );
    }
}

#[test]
fn the_two_sines_agree_to_within_rounding() {
    let by_time = sine(SAMPLE_RATE, 1.0, 440.0, 1.0);
    let by_count = sine_samples(SAMPLE_RATE, by_time.len(), 440.0, 1.0);

    for (a, b) in by_time.iter().zip(&by_count) {
        assert!((a - b).abs() <= 1e-6, "{a} vs {b}");
    }
}

/// A signal of time, with its name.
type Named = (&'static str, fn(f64) -> f64);

/// Every signal of time, at 0.1 s.
fn signals_at_a_tenth() -> [Named; 7] {
    [
        ("click", |t| click(t, 0.1, 0.1)),
        ("slow_click", |t| slow_click(t, 0.1, 0.1)),
        ("note", |t| note(t, 0.1, 82.4, 0.3)),
        ("bright_note", |t| bright_note(t, 0.1, 0.5, 1.0)),
        ("pluck", |t| pluck(t, 0.1)),
        ("quick_pluck", |t| quick_pluck(t, 0.1)),
        ("broadband_note", |t| {
            broadband_note(t, 0.1, &broadband_partials(16))
        }),
    ]
}

#[test]
fn every_signal_is_silent_before_it_starts_and_sounds_after() {
    for (name, signal) in signals_at_a_tenth() {
        let samples = synthesize(SAMPLE_RATE, 0.15, signal);
        let start = 4_800;

        assert!(
            samples[..start].iter().all(|&s| s == 0.0),
            "{name} sounds early"
        );
        assert!(
            peak(&samples[start..]) > 0.01,
            "{name} is silent: {}",
            peak(&samples[start..])
        );
    }
}

#[test]
fn a_click_dies_away_within_a_few_tens_of_milliseconds() {
    for (name, signal) in [
        ("click", click as fn(f64, f64, f64) -> f64),
        ("slow_click", slow_click),
    ] {
        let samples = synthesize(SAMPLE_RATE, 0.1, |t| signal(t, 0.0, 0.1));

        let tail = &samples[seconds(0.04)..];
        assert!(
            peak(tail) < 1e-3 * peak(&samples),
            "{name}: {} of {}",
            peak(tail),
            peak(&samples)
        );
    }
}

#[test]
fn a_plucks_note_rings_on_after_its_click() {
    for (name, signal) in [
        ("pluck", pluck as fn(f64, f64) -> f64),
        ("quick_pluck", quick_pluck),
    ] {
        let samples = synthesize(SAMPLE_RATE, 0.5, |t| signal(t, 0.0));

        let later = &samples[seconds(0.2)..seconds(0.25)];
        assert!(rms(later) > 0.05, "{name}: {}", rms(later));
    }
}

#[test]
fn a_quick_plucks_note_dies_away_faster_than_a_plucks() {
    let pluck = synthesize(SAMPLE_RATE, 1.0, |t| pluck(t, 0.0));
    let quick = synthesize(SAMPLE_RATE, 1.0, |t| quick_pluck(t, 0.0));
    let ratio = |signal: &[f32]| {
        rms(&signal[seconds(0.8)..seconds(0.9)]) / rms(&signal[seconds(0.1)..seconds(0.2)])
    };

    assert!(ratio(&quick) < ratio(&pluck));
}

#[test]
fn a_notes_level_rises_over_two_milliseconds() {
    let samples = synthesize(SAMPLE_RATE, 0.01, |t| note(t, 0.0, 1_000.0, 1.0));

    let rising = peak(&samples[..seconds(0.000_5)]);
    let risen = peak(&samples[seconds(0.002)..seconds(0.003)]);
    assert!(rising < 0.5 * risen, "{rising} vs {risen}");
}

#[test]
fn the_broadband_partials_span_50_hz_to_8_khz_evenly_in_pitch() {
    let partials = broadband_partials(120);

    assert_eq!(partials.len(), 120);
    assert!((partials[0].0 - 50.0).abs() < 1e-9);
    assert!((partials[119].0 - 8_000.0).abs() < 1e-6);
    let ratio = partials[1].0 / partials[0].0;
    for pair in partials.windows(2) {
        assert!((pair[1].0 / pair[0].0 - ratio).abs() < 1e-9);
    }
    assert!(
        partials
            .iter()
            .all(|&(_, phase)| (0.0..core::f64::consts::TAU).contains(&phase))
    );
}

#[test]
fn a_broadband_notes_phases_follow_time_not_its_start() {
    // Two notes picked at different times carry the same tone, so their ratio is only the ratio
    // of their envelopes.
    let partials = broadband_partials(120);
    for t in [0.3, 0.317, 0.5] {
        let ratio = broadband_note(t, 0.1, &partials) / broadband_note(t, 0.2, &partials);

        assert!(
            (ratio - (-0.1_f64 / 0.4).exp()).abs() < 1e-9,
            "{ratio} at {t} s"
        );
    }
}

#[test]
fn the_generator_gives_the_same_values_from_the_same_seed() {
    let mut first = Xorshift64::new(42);
    let mut second = Xorshift64::new(42);
    let mut other = Xorshift64::new(43);

    let a: Vec<u64> = (0..100).map(|_| first.next_u64()).collect();
    let b: Vec<u64> = (0..100).map(|_| second.next_u64()).collect();
    let c: Vec<u64> = (0..100).map(|_| other.next_u64()).collect();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn the_first_value_is_one_xorshift_step_from_the_seed() {
    let seed = 1_u64;
    let mut expected = seed;
    expected ^= expected << 13;
    expected ^= expected >> 7;
    expected ^= expected << 17;

    assert_eq!(Xorshift64::new(seed).next_u64(), expected);
}

#[test]
fn uniform_values_stay_in_their_range_and_fill_it() {
    let mut random = Xorshift64::new(0x9e37_79b9_7f4a_7c15);
    let values: Vec<f64> = (0..10_000).map(|_| random.uniform(0.12, 0.16)).collect();

    assert!(values.iter().all(|v| (0.12..0.16).contains(v)));
    let mean = values.iter().sum::<f64>() / 10_000.0;
    assert!((mean - 0.14).abs() < 0.001, "{mean}");
}

#[test]
fn noise_stays_in_range_with_a_mean_near_zero() {
    let streams: [Vec<f32>; 2] = [
        noise(7).take(48_000).collect(),
        noise32(0x1234_5678).take(48_000).collect(),
    ];
    for samples in streams {
        assert!(samples.iter().all(|s| (-1.0..1.0).contains(s)));
        let mean = samples.iter().map(|&s| f64::from(s)).sum::<f64>() / 48_000.0;
        assert!(mean.abs() < 0.02, "{mean}");
        // Uniform noise over -1..1 has an RMS of 1 / sqrt(3).
        assert!((rms(&samples) - 1.0 / 3_f64.sqrt()).abs() < 0.01);
    }
}

#[test]
fn noise_seeds_that_differ_only_in_the_lowest_bit_give_the_same_stream() {
    let even: Vec<f32> = noise(10).take(100).collect();
    let odd: Vec<f32> = noise(11).take(100).collect();
    let other: Vec<f32> = noise(12).take(100).collect();

    assert_eq!(even, odd);
    assert_ne!(even, other);
}

#[test]
fn rms_and_peak_of_silence_or_nothing_are_zero() {
    assert_eq!(rms(&[]), 0.0);
    assert_eq!(peak(&[]), 0.0);
    assert_eq!(rms(&[0.0; 10]), 0.0);
    assert_eq!(peak(&[0.0; 10]), 0.0);
}

#[test]
fn peak_counts_negative_samples() {
    assert_eq!(peak(&[0.1, -0.7, 0.5]), 0.7);
    assert!((rms(&[1.0, -1.0]) - 1.0).abs() < f64::EPSILON);
}

/// `duration` in samples at [`SAMPLE_RATE`].
fn seconds(duration: f64) -> usize {
    family_dsp::convert::seconds_to_samples(duration, SAMPLE_RATE)
}
