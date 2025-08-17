use crate::common::{PROGRAM_NAME, VizUdpClient, VizUdpServer};

use std::{
    net::{SocketAddr, ToSocketAddrs},
    process::{Child, Command},
};

// Based on the max number of bytes that can somewhat reliably be sent over UDP (508) divided by 4
// as we'll be sending f32's. That gives us 127, but since we're actually sending pairs of f32's,
// reduce this to an even number.
const MAX_SAMPLES_TO_SEND: usize = 126;

pub struct Config {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub max_num_samples: usize,
    pub line_width: u32,
    pub alpha_scale: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            scale: 500.0,
            max_num_samples: 5000,
            line_width: 1,
            alpha_scale: 255,
            red: 0,
            green: 255,
            blue: 0,
        }
    }
}

fn samples_to_ne_bytes(samples: &[f32], out: &mut Vec<u8>) {
    out.clear();
    for sample in samples {
        out.extend_from_slice(&sample.to_ne_bytes());
    }
}

fn samples_from_ne_bytes(bytes: &[u8], out: &mut Vec<f32>) {
    if bytes.len() % 4 != 0 {
        log::error!(
            "Received a buffer of bytes that is not a multiple of 4 bytes in length (length is {}).",
            bytes.len()
        );
    }
    out.clear();
    let mut buf = [0; 4];
    for w in bytes.chunks_exact(4) {
        buf.copy_from_slice(w);
        out.push(f32::from_ne_bytes(buf));
    }
}

fn start_client_app(
    server_addr: SocketAddr,
    program_name: &str,
    config: &Config,
) -> anyhow::Result<Child> {
    let mut command = Command::new(program_name);
    command.args([
        format!("--server={}", server_addr.to_string()),
        "oscilloscope".to_string(),
        format!("--width={}", config.width),
        format!("--height={}", config.height),
        format!("--scale={}", config.scale),
        format!("--max-num-samples={}", config.max_num_samples),
        format!("--line-width={}", config.line_width),
        format!("--alpha-scale={}", config.alpha_scale),
        format!("--red={}", config.red),
        format!("--green={}", config.green),
        format!("--blue={}", config.blue),
    ]);
    Ok(command.spawn()?)
}

pub struct Server(VizUdpServer);

impl Server {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        Ok(Self(VizUdpServer::new(|server_addr| {
            let _client_process =
                start_client_app(server_addr, PROGRAM_NAME, &config)?;
            Ok(())
        })?))
    }

    pub fn send_samples(&mut self, samples: &[f32]) -> anyhow::Result<()> {
        for samples_chunk in samples.chunks(MAX_SAMPLES_TO_SEND) {
            samples_to_ne_bytes(samples_chunk, &mut self.0.buf);
            if !self.0.send_buf()? {
                break;
            }
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

    pub fn pairs(&self) -> impl Iterator<Item = (f32, f32)> {
        self.buf.chunks_exact(2).map(|c| (c[0], c[1]))
    }
}
