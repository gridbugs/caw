use caw_core::{CellF32, Stereo, StereoPair, cell_f32};
use caw_player::{
    PlayerConfig, PlayerVisualizationData, VisualizationDataPolicy, play_stereo,
};
use caw_viz_udp_app_lib::{SendStatus, oscilloscope};
use lazy_static::lazy_static;
use std::{sync::Mutex, thread, time::Duration};

pub type LiveStereoOut = StereoPair<CellF32>;

lazy_static! {
    static ref OUT: LiveStereoOut = Stereo::new_fn(|| cell_f32(0.0));
    static ref VISUALIZATION_DATA: Mutex<PlayerVisualizationData> =
        Default::default();
    static ref INITIALIZED: Mutex<bool> = Mutex::new(false);
}

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
    let make_viz = move || {
        oscilloscope::Server::new("CAW Synthesizer", config.clone()).unwrap()
    };
    let mut viz = make_viz();
    thread::spawn(move || {
        loop {
            match viz_data.with_and_clear(|buf| viz.send_samples(buf)) {
                Ok(SendStatus::Success) => (),
                Ok(SendStatus::Disconnected) => {
                    // re-open the visualization if it was closed
                    viz = make_viz();
                }
                Err(e) => eprintln!("{}", e),
            }
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
