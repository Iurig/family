//! The click statistic: the largest sample-to-sample jump in a stereo or mono signal.

use family_dsp::{Jump, largest_jump, largest_jump_mono};

#[test]
fn finds_the_largest_jump_on_either_side() {
    let left = [0.0, 0.1, 0.2, 0.3, 0.2];
    let right = [0.0, 0.0, -0.5, -0.4, -0.4];

    assert_eq!(
        largest_jump(&left, &right),
        Some(Jump {
            size: 0.5,
            sample: 2
        })
    );
}

#[test]
fn a_falling_jump_counts_as_much_as_a_rising_one() {
    let left = [0.5, -0.5, -0.4];
    let right = [0.0, 0.0, 0.9];

    assert_eq!(
        largest_jump(&left, &right),
        Some(Jump {
            size: 1.0,
            sample: 1
        })
    );
}

#[test]
fn the_earliest_of_equal_jumps_wins() {
    let left = [0.0, 0.0, 0.25];
    let right = [0.0, 0.25, 0.25];

    assert_eq!(
        largest_jump(&left, &right),
        Some(Jump {
            size: 0.25,
            sample: 1
        })
    );
}

#[test]
fn a_signal_shorter_than_two_samples_has_no_jump() {
    assert_eq!(largest_jump(&[0.3], &[0.3]), None);
    assert_eq!(largest_jump(&[], &[]), None);
}

#[test]
fn finds_the_largest_jump_in_one_channel() {
    let signal = [0.0, 0.1, -0.3, -0.2, 0.0];

    assert_eq!(
        largest_jump_mono(&signal),
        Some(Jump {
            size: 0.4,
            sample: 2
        })
    );
}

#[test]
fn the_earliest_of_equal_jumps_in_one_channel_wins() {
    let signal = [0.0, 0.25, 0.25, 0.0];

    assert_eq!(
        largest_jump_mono(&signal),
        Some(Jump {
            size: 0.25,
            sample: 1
        })
    );
}

#[test]
fn a_channel_shorter_than_two_samples_has_no_jump() {
    assert_eq!(largest_jump_mono(&[0.3]), None);
    assert_eq!(largest_jump_mono(&[]), None);
}

proptest::proptest! {
    #[test]
    fn one_channel_has_the_jump_of_that_channel_on_both_sides(
        signal in proptest::collection::vec(-1.0_f32..=1.0, 0..1_000),
    ) {
        proptest::prop_assert_eq!(largest_jump_mono(&signal), largest_jump(&signal, &signal));
    }

    #[test]
    fn one_channel_of_any_floats_has_the_jump_of_that_channel_on_both_sides(
        signal in proptest::collection::vec(proptest::num::f32::ANY, 0..100),
    ) {
        // `Jump` compares with `==`, which a NaN size never passes, so compare the bits.
        let bits = |jump: Option<Jump>| jump.map(|jump| (jump.size.to_bits(), jump.sample));
        proptest::prop_assert_eq!(
            bits(largest_jump_mono(&signal)),
            bits(largest_jump(&signal, &signal))
        );
    }
}
