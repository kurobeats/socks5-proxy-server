use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const SOCKS_VERSION: u8 = 0x05;
const SOCKS_AUTH_NONE: u8 = 0x00;
const SOCKS_CONNECT: u8 = 0x01;
const SOCKS_IPV4: u8 = 0x01;

fn handle_client(mut client: TcpStream) {
    // Negotiate the socks5 protocol
    let mut buf = [0; 2];
    client.read_exact(&mut buf).unwrap();
    assert_eq!(buf, [SOCKS_VERSION, 1]);
    client.write_all(&[SOCKS_VERSION, SOCKS_AUTH_NONE]).unwrap();

    // Read the request details
    client.read_exact(&mut buf).unwrap();
    assert_eq!(buf, [SOCKS_VERSION, SOCKS_CONNECT]);
    let mut addr_type = [0; 1];
    client.read_exact(&mut addr_type).unwrap();
    let addr = match addr_type[0] {
        SOCKS_IPV4 => {
            let mut addr = [0; 4];
            client.read_exact(&mut addr).unwrap();
            format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
        }
        _ => panic!("unsupported address type"),
    };
    let mut port = [0; 2];
    client.read_exact(&mut port).unwrap();
    let port = (port[0] as u16) << 8 | port[1] as u16;
    let target = format!("{}:{}", addr, port);

    // Connect to the target
    let mut target_stream = TcpStream::connect(target).unwrap();

    // Send the response
    client.write_all(&[SOCKS_VERSION, 0x00, 0x00, SOCKS_IPV4]).unwrap();
    let mut addr = target_stream.peer_addr().unwrap().ip().octets();
    client.write_all(&addr).unwrap();
    let port = target_stream.peer_addr().unwrap().port();
    client.write_all(&[port as u8, (port >> 8) as u8]).unwrap();

    // Copy data between the client and target
    let _ = thread::spawn(move || {
        let _ = client.copy(&mut target_stream);
    });
    let _ = thread::spawn(move || {
        let _ = target_stream.copy(&mut client);
    });
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").unwrap();

    // Accept incoming connections
    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_client(stream);
    }
}
