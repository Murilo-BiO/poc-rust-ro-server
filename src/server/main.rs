use std::collections::VecDeque;
use std::error::Error;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;

use network::PlayerSession;
use packet::PacketParser;
use systems::System;
use systems::auth::auth_system;



fn main() -> Result<(), Box<dyn Error>> {
    let thread_count = 2;
    let addr = "127.0.0.1:6900";
    let tcp_listener = Arc::new(TcpListener::bind(&addr)?);
    let connections: Arc<Mutex<VecDeque<PlayerSession>>> = Arc::new(Mutex::new(VecDeque::new()));
    let packet_parser = Arc::new(PacketParser::new("auth_packets.txt"));
    println!("Listening on: {}", addr);

    let mut threads = Vec::new();

    let listener = tcp_listener.clone();
    let conn = connections.clone();
    let listener_thread = thread::spawn(move || loop {
        let (socket, _) = listener.accept().expect("Failed to accept connection!");

        let player_session = PlayerSession::new(socket);
        println!("Player connected {:?}", &player_session);
        let mut sessions = conn.lock().unwrap();
        sessions.push_back(player_session);
        drop(sessions);
        sleep(Duration::from_micros(500));
    });
    threads.push(listener_thread);

    for n in 0..thread_count {
        let connections = connections.clone();
        let packet_parser = packet_parser.clone();
        
        let network_thread = thread::Builder::new().name(format!("Network Thread {}", n)).spawn(move || {
            let tick_duration = Duration::from_micros(500);

            loop {
                let mut sessions = connections.lock().unwrap();
                let mut session = match sessions.pop_front() {
                    Some(s) => s,
                    None => {
                        drop(sessions);
                        sleep(tick_duration);
                        continue;
                    }
                };
            
                drop(sessions);

                use network::UpdateResponse;
                match session.update_sockets() {
                    UpdateResponse::Ok(data) => {
                        session.recv_queue.extend(packet_parser.extract_packets(data.as_slice()));
                    },
                    UpdateResponse::NoContent => {}
                    _ => {
                        sleep(tick_duration);
                        continue;
                    },
                }

                let mut sessions = connections.lock().unwrap();
                session.transmit();
                sessions.push_back(session);
                drop(sessions);
                sleep(tick_duration);
            }
        });
        threads.push(network_thread.unwrap());
    }
    println!("Running a total of {} Network threads", threads.len());

    
    let systems: Arc<Vec<System>> = Arc::new(vec![
        auth_system,
    ]);
    let conn = connections.clone();
    let systems = systems.clone();
    let server_thread = thread::spawn(move || loop {
        let tick_duration = Duration::from_micros(5);
        let mut sessions = conn.lock().unwrap();

        for session in sessions.iter_mut() {
            while let Some(packet) = session.recv_queue.pop_front() {
                println!("Received packet {:#06X} with {} bytes", packet.packet_id, packet.length);
                use systems::SystemResult::*;
                systems.iter().any(|s| match s(session, &packet) {
                    Processed => true,
                    NotProcessed => false
                });
            }
        }
        drop(sessions);
        sleep(tick_duration);
    });
    threads.push(server_thread);

    for thread in threads {
        let _ = thread.join();
    }

    Ok(())
}
