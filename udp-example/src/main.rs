use std::{error::Error, thread};

fn main() -> Result<(), Box<dyn Error>> {
    let udp_client = std::net::UdpSocket::bind("127.0.0.1:0")?;
    let udp_server = std::net::UdpSocket::bind("127.0.0.1:0")?;
    let addr = udp_server.local_addr()?;
    udp_client.connect(addr)?;

    println!("client: {:?}", udp_client);
    println!("server: {:?}", udp_server);

    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let (sz, addr) = udp_server
            .recv_from(&mut buf)
            .expect("error reading from client");
        println!("read {sz} bytes from {addr}");
        udp_server.send_to(&buf[..sz], addr)
    });

    let sz = udp_client.send_to("hello, world".as_bytes(), addr)?;
    println!("wrote {sz} bytes to {addr}");

    let mut buf = [0u8; 1024];
    let (sz, addr) = udp_client
        .recv_from(&mut buf)
        .expect("error reading from client");
    println!("read {sz} bytes from {addr}");

    Ok(())
}
