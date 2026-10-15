//! `wav split` and `wav join`: converting between a stereo file and a mono pair, keeping the
//! sample format.
#![expect(
    clippy::unwrap_used,
    reason = "test helpers; a failure should fail the test"
)]

use std::path::Path;
use std::process::{Command, Output};

/// Runs `wav` with `args`, from inside `dir`.
fn wav(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wav"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure_mentioning(output: &Output, words: &str) {
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(words), "{stderr}");
}

/// Noise from a xorshift generator, as 64-bit values.
fn noise(seed: u64, count: usize) -> Vec<u64> {
    let mut state = seed | 1;
    (0..count)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        })
        .collect()
}

const fn spec(
    channels: u16,
    bits_per_sample: u16,
    sample_format: hound::SampleFormat,
) -> hound::WavSpec {
    hound::WavSpec {
        channels,
        sample_rate: 48_000,
        bits_per_sample,
        sample_format,
    }
}

/// Writes `count` frames of full-range noise in `spec`'s format.
fn write_noise(path: &Path, spec: hound::WavSpec, count: usize, seed: u64) {
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for value in noise(seed, count * usize::from(spec.channels)) {
        match spec.sample_format {
            hound::SampleFormat::Float => {
                let unit = f32::from(u16::try_from(value >> 48).unwrap()) / 32_768.0 - 1.0;
                writer.write_sample(unit).unwrap();
            }
            hound::SampleFormat::Int => {
                let bits = u32::from(spec.bits_per_sample);
                let sample = i64::try_from(value >> (64 - bits)).unwrap() - (1 << (bits - 1));
                writer.write_sample(i32::try_from(sample).unwrap()).unwrap();
            }
        }
    }
    writer.finalize().unwrap();
}

/// A file's format and samples, as raw bits so floats compare exactly.
fn read(path: &Path) -> (hound::WavSpec, Vec<u32>) {
    let mut reader = hound::WavReader::open(path).unwrap();
    let spec = reader.spec();
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.unwrap().to_bits())
            .collect(),
        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.unwrap().cast_unsigned())
            .collect(),
    };
    (spec, samples)
}

const FORMATS: [(u16, hound::SampleFormat); 3] = [
    (16, hound::SampleFormat::Int),
    (24, hound::SampleFormat::Int),
    (32, hound::SampleFormat::Float),
];

#[test]
fn split_then_join_reproduces_the_stereo_file_exactly() {
    for (bits, format) in FORMATS {
        let dir = tempfile::tempdir().unwrap();
        write_noise(
            &dir.path().join("stereo.wav"),
            spec(2, bits, format),
            4_800,
            1,
        );

        assert_success(&wav(dir.path(), &["split", "stereo.wav", "L.wav", "R.wav"]));
        assert_success(&wav(dir.path(), &["join", "L.wav", "R.wav", "joined.wav"]));

        let original = std::fs::read(dir.path().join("stereo.wav")).unwrap();
        let joined = std::fs::read(dir.path().join("joined.wav")).unwrap();
        assert!(original == joined, "{bits}-bit {format:?}");
    }
}

#[test]
fn split_writes_each_side_as_mono_in_the_same_format() {
    for (bits, format) in FORMATS {
        let dir = tempfile::tempdir().unwrap();
        write_noise(
            &dir.path().join("stereo.wav"),
            spec(2, bits, format),
            100,
            2,
        );

        assert_success(&wav(dir.path(), &["split", "stereo.wav", "L.wav", "R.wav"]));

        let (_, stereo) = read(&dir.path().join("stereo.wav"));
        let (left_spec, left) = read(&dir.path().join("L.wav"));
        let (right_spec, right) = read(&dir.path().join("R.wav"));
        assert_eq!(left_spec, spec(1, bits, format));
        assert_eq!(right_spec, spec(1, bits, format));
        assert_eq!(left, stereo.iter().copied().step_by(2).collect::<Vec<_>>());
        assert_eq!(
            right,
            stereo
                .iter()
                .copied()
                .skip(1)
                .step_by(2)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn join_pads_the_shorter_side_with_silence() {
    let dir = tempfile::tempdir().unwrap();
    let mono = spec(1, 24, hound::SampleFormat::Int);
    write_noise(&dir.path().join("L.wav"), mono, 10, 3);
    write_noise(&dir.path().join("R.wav"), mono, 7, 4);

    assert_success(&wav(dir.path(), &["join", "L.wav", "R.wav", "joined.wav"]));

    let (_, left) = read(&dir.path().join("L.wav"));
    let (_, right) = read(&dir.path().join("R.wav"));
    let (joined_spec, joined) = read(&dir.path().join("joined.wav"));
    assert_eq!(joined_spec, spec(2, 24, hound::SampleFormat::Int));
    let padded_right = right.iter().copied().chain([0; 3]);
    let expected: Vec<u32> = left
        .iter()
        .copied()
        .zip(padded_right)
        .flat_map(<[u32; 2]>::from)
        .collect();
    assert_eq!(joined, expected);
}

#[test]
fn join_refuses_mismatched_sample_rates() {
    let dir = tempfile::tempdir().unwrap();
    let mono = spec(1, 24, hound::SampleFormat::Int);
    write_noise(&dir.path().join("L.wav"), mono, 10, 5);
    write_noise(
        &dir.path().join("R.wav"),
        hound::WavSpec {
            sample_rate: 44_100,
            ..mono
        },
        10,
        6,
    );

    let output = wav(dir.path(), &["join", "L.wav", "R.wav", "joined.wav"]);

    assert_failure_mentioning(&output, "sample rate");
    assert!(!dir.path().join("joined.wav").exists());
}

#[test]
fn join_refuses_mismatched_sample_formats() {
    let dir = tempfile::tempdir().unwrap();
    write_noise(
        &dir.path().join("L.wav"),
        spec(1, 24, hound::SampleFormat::Int),
        10,
        7,
    );
    write_noise(
        &dir.path().join("R.wav"),
        spec(1, 16, hound::SampleFormat::Int),
        10,
        8,
    );

    let output = wav(dir.path(), &["join", "L.wav", "R.wav", "joined.wav"]);

    assert_failure_mentioning(&output, "format");
}

#[test]
fn split_and_join_refuse_the_wrong_channel_counts() {
    let dir = tempfile::tempdir().unwrap();
    let int = hound::SampleFormat::Int;
    write_noise(&dir.path().join("mono.wav"), spec(1, 16, int), 10, 9);
    write_noise(&dir.path().join("stereo.wav"), spec(2, 16, int), 10, 10);

    let split = wav(dir.path(), &["split", "mono.wav", "L.wav", "R.wav"]);
    let join = wav(
        dir.path(),
        &["join", "stereo.wav", "mono.wav", "joined.wav"],
    );

    assert_failure_mentioning(&split, "stereo");
    assert_failure_mentioning(&join, "mono");
}
