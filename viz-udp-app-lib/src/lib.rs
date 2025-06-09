use std::{
    net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    process::{Child, Command},
};

pub const HANDSHAKE: [u8; 4] = [0x43, 0x41, 0x57, 0x0];

// Based on the max number of bytes that can somewhat reliably be sent over UDP (508) divided by 4
// as we'll be sending f32's. That gives us 127, but since we're actually sending pairs of f32's,
// reduce this to an even number.
const MAX_SAMPLES_TO_SEND: usize = 126;

const PROGRAM_NAME: &str = "caw_viz_udp_app";

pub struct VizAppConfig {
    pub program_name: String,
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

impl Default for VizAppConfig {
    fn default() -> Self {
        Self {
            program_name: PROGRAM_NAME.to_string(),
            width: 640,
            height: 480,
            scale: 1000.0,
            max_num_samples: 5000,
            line_width: 1,
            alpha_scale: 255,
            red: 0,
            green: 255,
            blue: 0,
        }
    }
}

fn wait_for_handshake(socket: &UdpSocket) -> anyhow::Result<SocketAddr> {
    let mut buf = [0; 8];
    loop {
        let (size, client_addr) = socket.recv_from(&mut buf)?;
        if size == HANDSHAKE.len() && buf[0..HANDSHAKE.len()] == HANDSHAKE {
            return Ok(client_addr);
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
    socket: &UdpSocket,
    config: &VizAppConfig,
) -> anyhow::Result<Child> {
    let mut command = Command::new(&config.program_name);
    command.args([
        format!("--server={}", socket.local_addr()?.to_string()),
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

pub struct VizUdpServer {
    socket: UdpSocket,
    buf: Vec<u8>,
    client_running: bool,
}

impl VizUdpServer {
    pub fn new(config: VizAppConfig) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        log::info!("Viz udp server address: {:?}", socket.local_addr());
        let _client_process = start_client_app(&socket, &config)?;
        let client_address = wait_for_handshake(&socket)?;
        log::info!("Viz udp client address: {:?}", client_address);
        socket.connect(client_address)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            buf: Vec::new(),
            client_running: true,
        })
    }

    pub fn send_samples(&mut self, samples: &[f32]) -> anyhow::Result<()> {
        if !self.client_running {
            // If the client window is closed then there is nothing to do. Possibly we could
            // restart the client at this point in the future?
            return Ok(());
        }
        for samples_chunk in samples.chunks(MAX_SAMPLES_TO_SEND) {
            samples_to_ne_bytes(samples_chunk, &mut self.buf);
            match self.socket.send(&self.buf) {
                Ok(bytes_sent) => {
                    if bytes_sent != self.buf.len() {
                        log::error!(
                            "Failed to send all samples to viz udp client. Tried to send {} bytes. Actually sent {} bytes.",
                            self.buf.len(),
                            bytes_sent
                        );
                    }
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::ConnectionRefused => {
                        self.client_running = false;
                        log::error!("Viz udp client has closed!");
                        break;
                    }
                    _ => anyhow::bail!(e),
                },
            }
        }
        Ok(())
    }
}

pub struct VizUdpClient {
    socket: UdpSocket,
    buf_raw: Vec<u8>,
    buf: Vec<f32>,
}

impl VizUdpClient {
    pub fn new<A: ToSocketAddrs>(server_address: A) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.connect(server_address)?;
        assert_eq!(socket.send(&HANDSHAKE)?, HANDSHAKE.len());
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            buf_raw: vec![0; 32768],
            buf: Vec::new(),
        })
    }

    pub fn recv_sample(&mut self) -> anyhow::Result<bool> {
        match self.socket.recv(&mut self.buf_raw) {
            Ok(size) => {
                samples_from_ne_bytes(&self.buf_raw[0..size], &mut self.buf);
                Ok(true)
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => Ok(false),
                _ => anyhow::bail!(e),
            },
        }
    }

    pub fn pairs(&self) -> impl Iterator<Item = (f32, f32)> {
        self.buf.chunks_exact(2).map(|c| (c[0], c[1]))
    }
}
