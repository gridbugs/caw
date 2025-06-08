use caw_viz_udp_app_lib::VizUdpServer;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let mut server = VizUdpServer::new()?;
    loop {
        server.send_samples(&[1.0, 1.5, 2.0, 3.0])?;
        std::thread::sleep(Duration::from_millis(100));
    }
}
