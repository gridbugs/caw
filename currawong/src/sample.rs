use currawong_core::sampler::Sample;
use hound::WavReader;
use std::{fs, io::BufReader, path::Path};

fn parse_wav(buffer: &[u8]) -> Vec<f64> {
    let mut reader = WavReader::new(BufReader::new(buffer)).unwrap();
    let spec = reader.spec();
    let max_value = (1 << (spec.bits_per_sample - 1)) as i64;
    let data_int = reader
        .samples::<i32>()
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    let data_f64 = data_int
        .chunks(spec.channels as usize)
        .map(|chunk| {
            let channel_mean = chunk.iter().map(|&x| x as i64).sum::<i64>() / chunk.len() as i64;
            (channel_mean as f64 / max_value as f64) as f64
        })
        .collect::<Vec<_>>();
    data_f64
}

pub fn read_wav(path: impl AsRef<Path>) -> anyhow::Result<Sample> {
    let raw = fs::read(path)?;
    let wav_samples = parse_wav(&raw);
    Ok(Sample::new(wav_samples))
}
