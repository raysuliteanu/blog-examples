use std::{error::Error, net::UdpSocket, thread};

fn main() -> Result<(), Box<dyn Error>> {
    let udp_client = UdpSocket::bind("127.0.0.1:0")?;
    let udp_server = UdpSocket::bind("127.0.0.1:0")?;
    let addr = udp_server.local_addr()?;
    udp_client.connect(addr)?;

    println!("client: {:?}", udp_client);
    println!("server: {:?}", udp_server);

    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let (sz, addr) = udp_server
            .recv_from(&mut buf)
            .expect("error reading from client");
        println!("server: read {sz} bytes");
        udp_server.send_to(&buf[..sz], addr)
    });

    let sz = udp_client.send_to("hello, world".as_bytes(), addr)?;
    println!("client: wrote {sz} bytes");

    let mut buf = [0u8; 1024];
    let (sz, _addr) = udp_client
        .recv_from(&mut buf)
        .expect("error reading from client");
    println!("client: read {sz} bytes");

    Ok(())
}
