use network::PlayerSession;
use packet::{Packet, PacketParser};
use std::io::{self, Write};
use std::net::TcpStream;

use packets::auth::{PacketAcAcceptLogin2, PacketCaLogin};

fn main() -> io::Result<()> {
    // Connect to localhost on port 6900
    let mut stream = TcpStream::connect("127.0.0.1:6900")?;
    let parser = PacketParser::new("auth_packets.txt");

    // Send a message to the server
    let mut pkt = PacketCaLogin::new();
    let username = b"mpereti";
    let password = b"8509d0ea";
    pkt.username[..username.len()].copy_from_slice(username);
    pkt.password[..password.len()].copy_from_slice(password);
    pkt.client_type = 22;
    stream.write_all(pkt.serialize().unwrap().as_slice())?;

    stream.set_nonblocking(true)?;

    let mut session = PlayerSession::new(stream);

    loop {
        use network::UpdateResponse;
        match session.update_sockets() {
            UpdateResponse::Ok(data) => {
                session
                    .recv_queue
                    .extend(parser.extract_packets(data.as_slice()));
                println!("Hellow");
            }
            UpdateResponse::NoContent => {}
            _ => {
                std::thread::sleep(std::time::Duration::from_micros(500));
            }
        }

        println!("Packets {}", session.recv_queue.len());

        while let Some(packet) = session.recv_queue.pop_front() {
            println!(
                "Received response with packet {:#06X} with {} bytes",
                packet.packet_id, packet.length
            );

            match packet.packet_id {
                0x0AC4 => {
                    let pkt = packet.parse::<PacketAcAcceptLogin2>();
                    println!(" Packet: {:?}", pkt);
                }
                _ => {}
            }
        }

        // Add a delay or implement some logic to determine when to break out of the loop
        std::thread::sleep(std::time::Duration::from_micros(500));
    }
}
