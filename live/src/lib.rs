use caw_core::{Sig, SigBoxedVar, Stereo, StereoPair, svf32};
use caw_player::{
    ConfigOwned, PlayerVisualizationData, VisualizationDataPolicy, play_stereo,
    play_stereo_default,
};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref OUT: StereoPair<Sig<SigBoxedVar<f32>>> =
        Stereo::new_fn(|| svf32(0.0));
    static ref VISUALIZATION_DATA: Mutex<PlayerVisualizationData> =
        Default::default();
    static ref INITIALIZED: Mutex<bool> = Mutex::new(false);
}

pub fn live_stereo_visualized(
    visualization_data_policy: VisualizationDataPolicy,
) -> (StereoPair<Sig<SigBoxedVar<f32>>>, PlayerVisualizationData) {
    let mut initialized = INITIALIZED.lock().unwrap();
    if *initialized {
        return (OUT.clone(), VISUALIZATION_DATA.lock().unwrap().clone());
    }
    *initialized = true;
    let player = play_stereo(
        OUT.clone(),
        ConfigOwned {
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

pub fn live_stereo() -> StereoPair<Sig<SigBoxedVar<f32>>> {
    let mut initialized = INITIALIZED.lock().unwrap();
    if *initialized {
        return OUT.clone();
    }
    *initialized = true;
    let player = play_stereo_default(OUT.clone()).unwrap();
    std::mem::forget(player);
    OUT.clone()
}
