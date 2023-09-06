// An example of using `sample` to efficiently perform decent quality sample rate conversion on a
// WAV file entirely on the stack.

use std::path::Path;

use dasp::{
    interpolate::sinc::Sinc,
    ring_buffer,
    signal::{self},
    Sample, Signal,
};
use hound::WavReader;

pub fn read_audio_i16_16khz(wav_file: &Path) -> impl Iterator<Item = i16> {
    let reader = WavReader::open(wav_file).unwrap();

    // Get the wav spec and create a target with the new desired sample rate.
    let spec = reader.spec();

    dbg!(spec);

    // Read the interleaved samples and convert them to a signal.
    let samples = reader
        .into_samples()
        .filter_map(Result::ok)
        .map(i16::to_sample::<f64>);
    let signal = signal::from_interleaved_samples_iter(samples);

    // Convert the signal's sample rate using `Sinc` interpolation.
    let ring_buffer = ring_buffer::Fixed::from([[0.0]; 100]);
    let sinc = Sinc::new(ring_buffer);

    let new_signal = signal.from_hz_to_hz(sinc, spec.sample_rate as f64, 16000.0);
    new_signal
        .until_exhausted()
        .map(|frame| frame[0].to_sample::<i16>())
}
