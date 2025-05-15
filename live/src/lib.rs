use caw_core::{Sig, SigBoxedVar, Stereo, StereoPair, svf32};
use caw_player::play_stereo_default;
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref OUT: StereoPair<Sig<SigBoxedVar<f32>>> =
        Stereo::new_fn(|| svf32(0.0));
    static ref INITIALIZED: Mutex<bool> = Mutex::new(false);
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
