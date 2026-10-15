//! The click statistic: the largest sample-to-sample jump in a stereo signal.

/// A jump between two neighbouring samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Jump {
    /// The absolute difference between the two samples.
    pub size: f32,
    /// The index of the second sample, where the jump lands.
    pub sample: usize,
}

/// Finds the largest jump on either side, or `None` if the signal is shorter than two samples.
/// Of equally large jumps, the earliest wins. The sides should be the same length; the longer one
/// is cut short.
#[must_use]
pub fn largest_jump(left: &[f32], right: &[f32]) -> Option<Jump> {
    let mut largest: Option<Jump> = None;
    for (index, pair) in left.windows(2).zip(right.windows(2)).enumerate() {
        let (&[l_before, l_after], &[r_before, r_after]) = pair else {
            continue;
        };
        let size = libm::fabsf(l_after - l_before).max(libm::fabsf(r_after - r_before));
        if largest.is_none_or(|largest| size > largest.size) {
            largest = Some(Jump {
                size,
                sample: index + 1,
            });
        }
    }
    largest
}
