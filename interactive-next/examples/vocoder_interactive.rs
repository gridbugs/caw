use caw_audio_file::read_wav_stereo;
use caw_core_next::*;
use caw_interactive_next::*;
use caw_keyboard::*;
use caw_modules::*;
use clap::Parser;

fn filter_chain_robot(
    sig: impl SigT<Item = f32>,
    band_freq_hz: impl SigT<Item = f32>,
    width_ratio: impl SigT<Item = f32>,
    _index: usize,
    _carrier_freq_hz: impl SigT<Item = f32>,
    _resonance: impl SigT<Item = f32>,
) -> Sig<impl SigT<Item = f32>> {
    let band_freq_hz = sig_shared(band_freq_hz);
    let env = sig
        .filter(band_pass::centered::default(
            band_freq_hz.clone(),
            width_ratio,
        ))
        .filter(envelope_follower());
    let osc = super_saw(60.)
        .build()
        .filter(low_pass::moog_ladder(band_freq_hz).resonance(1.0));
    osc * env
}

fn _filter_chain(
    sig: impl SigT<Item = f32>,
    band_freq_hz: impl SigT<Item = f32>,
    width_ratio: impl SigT<Item = f32>,
    _index: usize,
    carrier_freq_hz: impl SigT<Item = f32>,
    _resonance: impl SigT<Item = f32>,
) -> Sig<impl SigT<Item = f32>> {
    let band_freq_hz = sig_shared(band_freq_hz);
    let env = sig
        .filter(band_pass::centered::default(
            band_freq_hz.clone(),
            width_ratio,
        ))
        .filter(envelope_follower());
    let osc = oscillator(Sine, band_freq_hz + Sig(carrier_freq_hz)).build();
    osc * env
}

fn filter_bank(
    sig: impl SigT<Item = f32>,
    band_freqs_hz: Vec<f32>,
    width_ratio: impl SigT<Item = f32>,
    carrier_freq_hz: impl SigT<Item = f32>,
    resonance: impl SigT<Item = f32>,
) -> Sig<impl SigT<Item = f32>> {
    let sig = sig_shared(sig);
    let carrier_freq_hz = sig_shared(carrier_freq_hz);
    let resonance = sig_shared(resonance);
    let width_ratio = sig_shared(width_ratio);
    band_freqs_hz
        .into_iter()
        .enumerate()
        .map(|(i, band_freq_hz)| {
            filter_chain_robot(
                sig.clone(),
                band_freq_hz,
                width_ratio.clone(),
                i,
                carrier_freq_hz.clone(),
                resonance.clone(),
            )
        })
        .sum()
}

fn sig(input: Input, audio_file_buf: Vec<f32>) -> Sig<impl SigT<Item = f32>> {
    let sig = sample_playback(audio_file_buf).build();
    let n_bands = 128;
    let n_bands_per_octave = 12;
    let min_band_freq_hz = 10.0;
    let band_freqs_hz = (0..n_bands)
        .map(|i| {
            2f32.powf(i as f32 / n_bands_per_octave as f32) * min_band_freq_hz
        })
        .collect::<Vec<_>>();
    let width_ratio = 0.5 / n_bands_per_octave as f32;
    let carrier_freq_hz = input
        .keyboard
        .opinionated_key_events(Note::B0)
        .mono_voice()
        .note
        .freq_hz();
    filter_bank(
        sig,
        band_freqs_hz,
        width_ratio,
        carrier_freq_hz,
        input.mouse.y_01(),
    ) * 10.0
}

#[derive(Parser, Debug)]
struct Args {
    wav_path: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .stride(4)
        .fade(true)
        .build();
    let input = window.input();
    let audio_file_buf = read_wav_stereo(args.wav_path)?;
    window.play_stereo(
        Stereo::new(
            sig(input.clone(), audio_file_buf.left),
            sig(input.clone(), audio_file_buf.right),
        ),
        Default::default(),
    )
}
