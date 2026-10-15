//! `split` and `join`: converting between a stereo file and a mono pair.
//!
//! The samples are copied as they are, in the input's sample format and bit depth, so a split and
//! a join give back the original file exactly.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, ensure};

use crate::stats::format_name;

/// Splits a stereo WAV into a mono file per side.
#[derive(Debug, clap::Args)]
pub struct SplitArgs {
    /// The stereo file.
    stereo: PathBuf,
    /// Where to write the left side.
    left: PathBuf,
    /// Where to write the right side.
    right: PathBuf,
}

/// Joins two mono WAVs into a stereo file. The shorter one is padded with silence.
#[derive(Debug, clap::Args)]
pub struct JoinArgs {
    /// The left side.
    left: PathBuf,
    /// The right side, at the same sample rate and in the same sample format.
    right: PathBuf,
    /// Where to write the stereo file.
    stereo: PathBuf,
}

/// A file's samples, interleaved, in their own format.
enum Samples {
    Int(Vec<i32>),
    Float(Vec<f32>),
}

/// Reads a file's format and samples, unconverted.
fn read(path: &Path) -> Result<(hound::WavSpec, Samples)> {
    let mut reader =
        hound::WavReader::open(path).with_context(|| format!("can't open {}", path.display()))?;
    let spec = reader.spec();
    let samples = match spec.sample_format {
        hound::SampleFormat::Int => reader.samples().collect::<Result<_, _>>().map(Samples::Int),
        hound::SampleFormat::Float => reader
            .samples()
            .collect::<Result<_, _>>()
            .map(Samples::Float),
    }
    .with_context(|| format!("can't read {}", path.display()))?;
    Ok((spec, samples))
}

/// Writes interleaved samples, in a format that matches them, to `path`.
fn write(path: &Path, spec: hound::WavSpec, samples: &Samples) -> Result<()> {
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("can't create {}", path.display()))?;
    match samples {
        Samples::Int(samples) => samples
            .iter()
            .try_for_each(|&sample| writer.write_sample(sample)),
        Samples::Float(samples) => samples
            .iter()
            .try_for_each(|&sample| writer.write_sample(sample)),
    }
    .and_then(|()| writer.finalize())
    .with_context(|| format!("can't write {}", path.display()))
}

/// Writes the left and right sides of a stereo file to two mono files.
///
/// # Errors
///
/// If the file isn't stereo, or a file can't be read or written.
pub fn split(args: &SplitArgs) -> Result<()> {
    let (spec, samples) = read(&args.stereo)?;
    ensure!(
        spec.channels == 2,
        "{} has {} channels; split takes a stereo file",
        args.stereo.display(),
        spec.channels
    );
    let mono = hound::WavSpec {
        channels: 1,
        ..spec
    };
    let (left, right) = match samples {
        Samples::Int(samples) => {
            let (left, right) = deinterleave(&samples);
            (Samples::Int(left), Samples::Int(right))
        }
        Samples::Float(samples) => {
            let (left, right) = deinterleave(&samples);
            (Samples::Float(left), Samples::Float(right))
        }
    };
    write(&args.left, mono, &left)?;
    write(&args.right, mono, &right)
}

/// Writes two mono files as the left and right sides of a stereo file.
///
/// # Errors
///
/// If the files aren't both mono in the same format, or a file can't be read or written.
pub fn join(args: &JoinArgs) -> Result<()> {
    let (left_spec, left) = read(&args.left)?;
    let (right_spec, right) = read(&args.right)?;
    for (path, spec) in [(&args.left, left_spec), (&args.right, right_spec)] {
        ensure!(
            spec.channels == 1,
            "{} has {} channels; join takes two mono files",
            path.display(),
            spec.channels
        );
    }
    ensure!(
        left_spec.sample_rate == right_spec.sample_rate,
        "{} and {} have different sample rates ({} and {} Hz)",
        args.left.display(),
        args.right.display(),
        left_spec.sample_rate,
        right_spec.sample_rate
    );
    ensure!(
        left_spec == right_spec,
        "{} and {} have different sample formats ({} and {})",
        args.left.display(),
        args.right.display(),
        format_name(left_spec),
        format_name(right_spec)
    );
    let stereo = match (left, right) {
        (Samples::Int(left), Samples::Int(right)) => Samples::Int(interleave(&left, &right)),
        (Samples::Float(left), Samples::Float(right)) => Samples::Float(interleave(&left, &right)),
        // The specs match, so the sample types do too.
        _ => anyhow::bail!("the files have different sample formats"),
    };
    write(
        &args.stereo,
        hound::WavSpec {
            channels: 2,
            ..left_spec
        },
        &stereo,
    )
}

/// Splits interleaved stereo samples into the left and right sides.
fn deinterleave<T: Copy>(samples: &[T]) -> (Vec<T>, Vec<T>) {
    let (frames, _) = samples.as_chunks::<2>();
    frames.iter().copied().map(<(T, T)>::from).unzip()
}

/// Interleaves two sides, padding the shorter with zeros (silence).
fn interleave<T: Copy + Default>(left: &[T], right: &[T]) -> Vec<T> {
    let length = left.len().max(right.len());
    let side = |samples: &[T], index| samples.get(index).copied().unwrap_or_default();
    (0..length)
        .flat_map(|index| [side(left, index), side(right, index)])
        .collect()
}
