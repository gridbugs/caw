use crate::common::{
    MAX_F32_SAMPLES_TO_SEND, PROGRAM_NAME, SendStatus, VizUdpClient,
    VizUdpServer, samples_from_ne_bytes, samples_to_ne_bytes,
};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    process::{Child, Command},
};

#[derive(Clone, Copy, Debug)]
pub enum OscilloscopeStyle {
    Xy,
    TimeDomain,
    TimeDomainStereo,
}
impl ToString for OscilloscopeStyle {
    fn to_string(&self) -> String {
        match self {
            OscilloscopeStyle::Xy => format!("xy"),
            OscilloscopeStyle::TimeDomain => format!("time-domain"),
            OscilloscopeStyle::TimeDomainStereo => {
                format!("time-domain-stereo")
            }
        }
    }
}

#[derive(Clone)]
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
    pub style: OscilloscopeStyle,
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
            style: OscilloscopeStyle::Xy,
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

    pub fn pairs(&self) -> impl Iterator<Item = (f32, f32)> {
        self.buf.chunks_exact(2).map(|c| (c[0], c[1]))
    }
}
