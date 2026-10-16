//! Averaging a stereo input to a DI.

/// Writes the mean of `left` and `right` to `di`, sample by sample.
///
/// For a plugin that only takes a DI, when the host feeds it a stereo signal: the sides are
/// averaged rather than one being dropped. Only the samples present in all three slices are
/// written; the rest of `di` is left as it was. The mean can't overflow, even of two full-scale
/// floats at the edge of the `f32` range.
pub fn average_to_di(left: &[f32], right: &[f32], di: &mut [f32]) {
    for ((&l, &r), out) in left.iter().zip(right).zip(di.iter_mut()) {
        *out = f32::midpoint(l, r);
    }
}
