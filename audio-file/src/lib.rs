use caw_core::Stereo;
use hound::WavReader;
use std::{fs, io::BufReader, path::Path};

fn parse_wav_mono(buffer: &[u8]) -> Vec<f32> {
    let mut reader = WavReader::new(BufReader::new(buffer)).unwrap();
    let spec = reader.spec();
    let max_value = (1 << (spec.bits_per_sample - 1)) as i64;
    let data_int = reader
        .samples::<i32>()
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    let data_f32 = data_int
        .chunks(spec.channels as usize)
        .map(|chunk| {
            let channel_mean = chunk.iter().map(|&x| x as i64).sum::<i64>()
                / chunk.len() as i64;
            channel_mean as f32 / max_value as f32
        })
        .collect::<Vec<_>>();
    data_f32
}

fn parse_wav_stereo(buffer: &[u8]) -> Stereo<Vec<f32>, Vec<f32>> {
    let mut reader = WavReader::new(BufReader::new(buffer)).unwrap();
    let spec = reader.spec();
    let max_value = (1 << (spec.bits_per_sample - 1)) as i64;
    let data_int = reader
        .samples::<i32>()
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    let mut out: Stereo<Vec<f32>, Vec<f32>> = Stereo::default();
    for chunk in data_int.chunks(spec.channels as usize) {
        let left = chunk[0];
        let right = chunk.get(1).cloned().unwrap_or(left);
        out.left.push(left as f32 / max_value as f32);
        out.right.push(right as f32 / max_value as f32);
    }
    out
}

pub fn read_wav_mono(path: impl AsRef<Path>) -> anyhow::Result<Vec<f32>> {
    let raw = fs::read(path)?;
    Ok(parse_wav_mono(&raw))
}

pub fn read_wav_stereo(
    path: impl AsRef<Path>,
) -> anyhow::Result<Stereo<Vec<f32>, Vec<f32>>> {
    let raw = fs::read(path)?;
    Ok(parse_wav_stereo(&raw))
}
