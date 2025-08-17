use caw_viz_udp_app_lib::oscilloscope;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let mut server = oscilloscope::Server::new(oscilloscope::Config {
        ..Default::default()
    })?;
    loop {
        server.send_samples(&[-0.5, -0.75, 1.0, 0.5])?;
        std::thread::sleep(Duration::from_millis(100));
    }
}
