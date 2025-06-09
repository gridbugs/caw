use caw::prelude::*;
use std::{thread, time::Duration};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let sig = Stereo::new_fn_channel(|channel| {
        let freq_hz = match channel {
            Channel::Left => 60.,
            Channel::Right => 90.5,
        };
        oscillator(Sine, freq_hz)
            .reset_offset_01(channel.circle_phase_offset_01())
            .build();
        super_saw(40.)
            .build()
            .filter(low_pass::default(6000.).q(0.5))
            .filter(chorus())
            .filter(reverb())
    });
    let player_handle = play_stereo(
        sig,
        PlayerConfig {
            visualization_data_policy: Some(VisualizationDataPolicy::All),
            ..Default::default()
        },
    )?;
    let viz_data = player_handle.visualization_data();
    let mut viz = VizUdpServer::new(VizAppConfig {
        scale: 1000.0,
        alpha_scale: 50,
        line_width: 2,
        max_num_samples: 10_000,
        ..Default::default()
    })?;
    loop {
        viz_data.with_and_clear(|buf| viz.send_samples(buf))?;
        thread::sleep(Duration::from_millis(16));
    }
}
