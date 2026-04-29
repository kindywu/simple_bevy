use std::net::UdpSocket;

fn main() {
    let socket = UdpSocket::bind("127.0.0.1:5000").expect("bind failed");
    println!("UDP server listening on 127.0.0.1:5000");

    let mut buf = [0u8; 1024];

    loop {
        let (len, addr) = socket.recv_from(&mut buf).expect("recv failed");
        println!("收到 {} bytes 来自 {}", len, addr);
    }
}
