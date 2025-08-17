use caw_core::{Sig, SigBoxedVar, Stereo, StereoPair, svf32};
use caw_player::{
    PlayerConfig, PlayerVisualizationData, VisualizationDataPolicy, play_stereo,
};
use caw_viz_udp_app_lib::oscilloscope;
use lazy_static::lazy_static;
use std::{sync::Mutex, thread, time::Duration};

lazy_static! {
    static ref OUT: StereoPair<Sig<SigBoxedVar<f32>>> =
        Stereo::new_fn(|| svf32(0.0));
    static ref VISUALIZATION_DATA: Mutex<PlayerVisualizationData> =
        Default::default();
    static ref INITIALIZED: Mutex<bool> = Mutex::new(false);
}

pub type LiveStereoOut = StereoPair<Sig<SigBoxedVar<f32>>>;

pub fn live_stereo_viz(
    visualization_data_policy: VisualizationDataPolicy,
) -> (LiveStereoOut, PlayerVisualizationData) {
    let mut initialized = INITIALIZED.lock().unwrap();
    if *initialized {
        return (OUT.clone(), VISUALIZATION_DATA.lock().unwrap().clone());
    }
    *initialized = true;
    let player = play_stereo(
        OUT.clone(),
        PlayerConfig {
            visualization_data_policy: Some(visualization_data_policy),
            ..Default::default()
        },
    )
    .unwrap();
    let mut visualization_data_ref = VISUALIZATION_DATA.lock().unwrap();
    *visualization_data_ref = player.visualization_data();
    std::mem::forget(player);
    (OUT.clone(), visualization_data_ref.clone())
}

pub fn live_stereo_viz_udp(config: oscilloscope::Config) -> LiveStereoOut {
    let (out, viz_data) = live_stereo_viz(VisualizationDataPolicy::All);
    let mut viz = oscilloscope::Server::new("CAW Synthesizer", config).unwrap();
    thread::spawn(move || {
        loop {
            // TODO: Is it ok to ignore errors here?
            let _ = viz_data.with_and_clear(|buf| viz.send_samples(buf));
            thread::sleep(Duration::from_millis(16));
        }
    });
    out
}

pub fn live_stereo() -> LiveStereoOut {
    let mut initialized = INITIALIZED.lock().unwrap();
    if *initialized {
        return OUT.clone();
    }
    *initialized = true;
    let player = play_stereo(OUT.clone(), Default::default()).unwrap();
    std::mem::forget(player);
    OUT.clone()
}
