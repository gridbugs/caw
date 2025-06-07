use std::{
    net::{Ipv4Addr, UdpSocket},
    process::Command,
    time::Duration,
};

// TODO
const PROGRAM_NAME: &str = "/home/s/src/caw/target/debug/caw_viz_udp_app";

fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    println!("Our address: {:?}", socket.local_addr()?);
    let mut command = Command::new(PROGRAM_NAME);
    command.args(["--server".to_string(), socket.local_addr()?.to_string()]);
    let _child = command.spawn()?;
    let mut buf = [0];
    let (size, client_addr) = socket.recv_from(&mut buf)?;
    // TODO: expect a magic number here
    assert_eq!(size, 1);
    assert_eq!(buf, [42]);
    println!("Client address: {:?}", client_addr);
    socket.connect(client_addr)?;
    let mut i = 0;
    loop {
        // TODO: gracefully shut down if the client dissapears
        assert_eq!(socket.send(&[i])?, 1);
        i = i.wrapping_add(1);
        std::thread::sleep(Duration::from_millis(100));
    }
}
