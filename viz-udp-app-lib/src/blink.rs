use crate::common::{
    MAX_F32_SAMPLES_TO_SEND, PROGRAM_NAME, SendStatus, VizUdpClient,
    VizUdpServer, samples_from_ne_bytes, samples_to_ne_bytes,
};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    process::{Child, Command},
};

#[derive(Clone, Debug)]
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
            red: 127,
            green: 0,
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
        "blink".to_string(),
        format!("--width={}", config.width),
        format!("--height={}", config.height),
        format!("--red={}", config.red),
        format!("--green={}", config.green),
        format!("--blue={}", config.blue),
    ]);
    Ok(command.spawn()?)
}

pub struct Server {
    raw: VizUdpServer,
    buf: Vec<f32>,
}

fn send_samples(
    server: &mut VizUdpServer,
    samples: &[f32],
) -> anyhow::Result<SendStatus> {
    for samples_chunk in samples.chunks(MAX_F32_SAMPLES_TO_SEND) {
        samples_to_ne_bytes(samples_chunk, &mut server.buf);
        if !server.send_buf()? {
            return Ok(SendStatus::Disconnected);
        }
    }
    Ok(SendStatus::Success)
}

impl Server {
    pub fn new(title: &str, config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            raw: VizUdpServer::new(|server_addr| {
                let _client_process = start_client_app(
                    server_addr,
                    PROGRAM_NAME,
                    title,
                    &config,
                )?;
                Ok(())
            })?,
            buf: Vec::new(),
        })
    }

    pub fn send_samples(
        &mut self,
        samples: &[f32],
    ) -> anyhow::Result<SendStatus> {
        send_samples(&mut self.raw, samples)
    }

    pub fn send_samples_batched(
        &mut self,
        samples: &[f32],
    ) -> anyhow::Result<()> {
        self.buf.extend_from_slice(samples);
        if self.buf.len() >= MAX_F32_SAMPLES_TO_SEND {
            send_samples(&mut self.raw, &self.buf)?;
            self.buf.clear();
        }
        Ok(())
    }
}

pub struct Client {
    raw: VizUdpClient,
    buf: Vec<f32>,
}

impl Client {
    pub fn new<A: ToSocketAddrs>(server_address: A) -> anyhow::Result<Self> {
        Ok(Self {
            raw: VizUdpClient::new(server_address)?,
            buf: Vec::new(),
        })
    }

    pub fn recv_sample(&mut self) -> anyhow::Result<bool> {
        Ok(match self.raw.recv()? {
            Some(buf_raw) => {
                samples_from_ne_bytes(buf_raw, &mut self.buf);
                true
            }
            None => false,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = f32> {
        self.buf.iter().cloned()
    }
}
