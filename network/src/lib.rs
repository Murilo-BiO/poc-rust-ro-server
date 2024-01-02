use packet::RawPacket;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Debug)]
pub enum UpdateResponse {
    Disconnected,
    Error,
    NoContent,
    Ok(Vec<u8>),
}

#[derive(Debug)]
pub struct PlayerSession {
    pub socket: TcpStream,
    pub recv_queue: VecDeque<RawPacket>,
    pub send_queue: VecDeque<Vec<u8>>
}

impl PlayerSession {
    pub fn new(socket: TcpStream) -> Self {
        socket
            .set_nonblocking(true)
            .expect("Failed to set player session in non-blocking mode");
        Self {
            socket,
            recv_queue: VecDeque::new(),
            send_queue: VecDeque::new(),
        }
    }

    pub fn update_sockets(&mut self) -> UpdateResponse {
        let mut buf = [0_u8; 1024];
        use std::io::ErrorKind::*;
        match self.socket.read(&mut buf) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    println!("PlayerSession disconnected {:?}", self.socket);
                    return UpdateResponse::Disconnected;
                }

                UpdateResponse::Ok(buf[..bytes_read].to_vec())
            },
            Err(ref e) if e.kind() == WouldBlock =>  UpdateResponse::NoContent,
            Err(err) => {
                eprintln!("Failed to read packets from player: {}", err);
                UpdateResponse::Error
            },
        }
    }

    pub fn transmit(&mut self) {
        while let Some(packet) = self.send_queue.pop_front() {
            println!("sending packet with len: {}", packet.len());
            self.socket.write_all(packet.as_slice()).unwrap();
        }
    }
}