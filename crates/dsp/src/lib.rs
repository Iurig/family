//! DSP shared by the family's plugins.
//!
//! This crate is `no_std` without `alloc`, like each plugin's own `dsp` crate, so the audio path
//! cannot allocate.
#![cfg_attr(not(test), no_std)]

pub mod convert;
mod jump;

pub use jump::{Jump, largest_jump};
