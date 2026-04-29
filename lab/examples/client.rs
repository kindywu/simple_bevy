use std::net::UdpSocket;

fn main() {
    let socket = UdpSocket::bind("127.0.0.1:0").expect("bind failed");

    let msg = b"hello";
    socket.send_to(msg, "127.0.0.1:5000").expect("send failed");

    println!("发送成功");
}
