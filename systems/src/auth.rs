use std::net::Ipv4Addr;

use packet::{Packet, RawPacket};
use network::PlayerSession;
use packets::auth::*;

use super::SystemResult::{self, *};

pub fn auth_system(session: &mut PlayerSession, packet: &RawPacket) -> SystemResult {
	let result = match packet.packet_id {
		0x0064 => packet.parse::<PacketCaLogin>()
			.map(|p| process_login(session, p)),
		_ => None, // if not processed it can be processed by another system
	};

	match result {
		Some(_) => Processed,
		None => NotProcessed
	}
}

fn process_login(session: &mut PlayerSession, _pkt: PacketCaLogin) {
	let mut accepted: PacketAcAcceptLogin2 = PacketAcAcceptLogin2::new();

	accepted.aid = 2000000;
	accepted.auth_code = 2000000;

	let mut server = CharServerList::default();
	server.ip = u32::from_be_bytes(Ipv4Addr::new(127, 0, 0, 1).octets());


	let v = b"Einbroch";
	server.name[..v.len()].copy_from_slice(v);

	server.port = 6121;
	server.usercount = 10;


	accepted.char_server_list.push(server);
	accepted.packet_len = accepted.len() as i16;
	match accepted.serialize() {
		Some(buf) => {
			session.send_queue.push_back(buf);
			println!("Added packet to the send list of session '{:?}'", session.socket.peer_addr());
		},
		None => {
			println!("Couldn't serialize packet! {:?}", accepted);
		}
	};
}
