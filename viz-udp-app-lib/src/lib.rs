/// A wrapper for the `caw_viz_udp_app` command. That command starts a udp client and opens a
/// window. The client listens for messages from a server representing vizualizations and renders
/// them in the window. This library provides the server implementation. Synthesizer code is
/// expected to use this library to create a udp server and spawn a client which connects to it,
/// and then manipulate the server to send visualization commands to the client. This library also
/// contains helper code for writing client applications.
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};

pub const HANDSHAKE: [u8; 4] = [0x43, 0x41, 0x57, 0x0];

const PROGRAM_NAME: &str = "caw_viz_udp_app";

fn wait_for_handshake(socket: &UdpSocket) -> anyhow::Result<SocketAddr> {
    let mut buf = [0; 8];
    loop {
        let (size, client_addr) = socket.recv_from(&mut buf)?;
        if size == HANDSHAKE.len() && buf[0..HANDSHAKE.len()] == HANDSHAKE {
            return Ok(client_addr);
        }
    }
}

struct VizUdpServer {
    socket: UdpSocket,
    buf: Vec<u8>,
    client_running: bool,
}

impl VizUdpServer {
    pub fn new<F: FnOnce(SocketAddr) -> anyhow::Result<()>>(
        start_client: F,
    ) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        log::info!("Viz udp server address: {:?}", socket.local_addr());
        start_client(socket.local_addr()?)?;
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

    /// Returns `true` iff it succeeded in sending the data and `false` if the client has
    /// disconnected.
    pub fn send_buf(&mut self) -> anyhow::Result<bool> {
        if !self.client_running {
            // If the client window is closed then there is nothing to do. Possibly we could
            // restart the client at this point in the future?
            return Ok(false);
        }
        match self.socket.send(&self.buf) {
            Ok(bytes_sent) => {
                if bytes_sent != self.buf.len() {
                    log::error!(
                        "Failed to send all data to viz udp client. Tried to send {} bytes. Actually sent {} bytes.",
                        self.buf.len(),
                        bytes_sent
                    );
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::ConnectionRefused => {
                    self.client_running = false;
                    log::error!("Viz udp client has closed!");
                }
                _ => anyhow::bail!(e),
            },
        }
        Ok(true)
    }
}

pub struct VizUdpClient {
    socket: UdpSocket,
    buf: Vec<u8>,
}

impl VizUdpClient {
    pub fn new<A: ToSocketAddrs>(server_address: A) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.connect(server_address)?;
        assert_eq!(socket.send(&HANDSHAKE)?, HANDSHAKE.len());
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            buf: vec![0; 32768],
        })
    }

    /// Returns `Some(buf)` containing a buffer of bytes received, or `None` if no bytes are
    /// currently available.
    pub fn recv(&mut self) -> anyhow::Result<Option<&[u8]>> {
        match self.socket.recv(&mut self.buf) {
            Ok(size) => Ok(Some(&self.buf[0..size])),
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => Ok(None),
                _ => anyhow::bail!(e),
            },
        }
    }
}

pub mod oscilloscope {

    use super::{PROGRAM_NAME, VizUdpClient, VizUdpServer};

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
            "oscilloscope".to_string(),
            format!("--server={}", server_addr.to_string()),
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
        pub fn new<A: ToSocketAddrs>(
            server_address: A,
        ) -> anyhow::Result<Self> {
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
}
