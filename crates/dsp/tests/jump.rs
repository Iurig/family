//! The click statistic: the largest sample-to-sample jump in a stereo signal.

use family_dsp::{Jump, largest_jump};

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
