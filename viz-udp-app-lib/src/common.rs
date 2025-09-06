use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
pub const HANDSHAKE: [u8; 4] = [0x43, 0x41, 0x57, 0x0];

pub const PROGRAM_NAME: &str = "caw_viz_udp_app";

// Based on the max number of bytes that can somewhat reliably be sent over UDP (508) divided by 4
// as we'll be sending f32's. That gives us 127, but since we're actually sending pairs of f32's,
// reduce this to an even number.
pub const MAX_F32_SAMPLES_TO_SEND: usize = 126;

pub fn samples_to_ne_bytes(samples: &[f32], out: &mut Vec<u8>) {
    out.clear();
    for sample in samples {
        out.extend_from_slice(&sample.to_ne_bytes());
    }
}

pub fn samples_from_ne_bytes(bytes: &[u8], out: &mut Vec<f32>) {
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

fn wait_for_handshake(socket: &UdpSocket) -> anyhow::Result<SocketAddr> {
    let mut buf = [0; 8];
    loop {
        let (size, client_addr) = socket.recv_from(&mut buf)?;
        if size == HANDSHAKE.len() && buf[0..HANDSHAKE.len()] == HANDSHAKE {
            return Ok(client_addr);
        }
    }
}

pub struct VizUdpServer {
    socket: UdpSocket,
    pub buf: Vec<u8>,
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
    pub buf: Vec<u8>,
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
