//! Averaging a stereo input to a DI, for hosts that feed a plugin a stereo signal.

use family_dsp::average_to_di;

#[test]
fn each_di_sample_is_the_mean_of_left_and_right() {
    let left = [1.0, 0.5, -1.0, 0.0];
    let right = [0.0, 0.5, 1.0, -0.5];
    let mut di = [9.0; 4];

    average_to_di(&left, &right, &mut di);

    assert_eq!(di, [0.5, 0.5, 0.0, -0.25]);
}

#[test]
fn only_the_overlapping_samples_are_written() {
    let left = [1.0, 1.0, 1.0];
    let right = [1.0, 1.0];
    let mut di = [9.0; 4];

    average_to_di(&left, &right, &mut di);

    assert_eq!(di, [1.0, 1.0, 9.0, 9.0]);
}

#[test]
fn a_shorter_di_takes_only_the_samples_it_has_room_for() {
    let left = [1.0, 0.0, -1.0];
    let right = [0.0, 0.0, -1.0];
    let mut di = [9.0; 2];

    average_to_di(&left, &right, &mut di);

    assert_eq!(di, [0.5, 0.0]);
}

#[test]
fn the_mean_of_the_largest_floats_does_not_overflow() {
    let left = [f32::MAX, f32::MIN];
    let right = [f32::MAX, f32::MIN];
    let mut di = [0.0; 2];

    average_to_di(&left, &right, &mut di);

    assert_eq!(di, [f32::MAX, f32::MIN]);
}

proptest::proptest! {
    #[test]
    fn the_same_signal_on_both_sides_averages_to_that_signal(
        signal in proptest::collection::vec(-1.0_f32..=1.0, 0..1_000),
    ) {
        let mut di = vec![0.0; signal.len()];

        average_to_di(&signal, &signal, &mut di);

        proptest::prop_assert_eq!(di, signal);
    }

    #[test]
    fn an_inverted_copy_averages_to_silence(
        signal in proptest::collection::vec(-1.0_f32..=1.0, 0..1_000),
    ) {
        let inverted: Vec<f32> = signal.iter().map(|&s| -s).collect();
        let mut di = vec![9.0; signal.len()];

        average_to_di(&signal, &inverted, &mut di);

        proptest::prop_assert!(di.iter().all(|&s| s == 0.0), "{di:?}");
    }
}
