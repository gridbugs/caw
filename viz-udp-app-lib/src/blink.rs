use crate::common::{
    MAX_F32_SAMPLES_TO_SEND, PROGRAM_NAME, VizUdpClient, VizUdpServer,
};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    process::{Child, Command},
};

pub struct Config {
    pub width: u32,
    pub height: u32,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 100,
            height: 100,
            red: 0,
            green: 255,
            blue: 0,
        }
    }
}

fn start_client_app(
    server_addr: SocketAddr,
    program_name: &str,
    title: &str,
    config: &Config,
) -> anyhow::Result<Child> {
    let mut command = Command::new(program_name);
    command.args([
        format!("--server={}", server_addr.to_string()),
        format!("--title={}", title.to_string()),
        "oscilloscope".to_string(),
        format!("--width={}", config.width),
        format!("--height={}", config.height),
        format!("--red={}", config.red),
        format!("--green={}", config.green),
        format!("--blue={}", config.blue),
    ]);
    Ok(command.spawn()?)
}
