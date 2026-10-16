//! DSP shared by the family's plugins.
//!
//! This crate is `no_std` without `alloc`, like each plugin's own `dsp` crate, so the audio path
//! cannot allocate.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(
    test,
    expect(
        clippy::suboptimal_flops,
        reason = "only test builds have std's `mul_add`"
    )
)]

mod average;
pub mod convert;
mod crossover;
mod jump;

pub use average::average_to_di;
pub use crossover::{BUTTERWORTH_Q, Biquad, Crossover, Steepness, UnsupportedSteepness, prewarp};
pub use jump::{Jump, largest_jump, largest_jump_mono};
