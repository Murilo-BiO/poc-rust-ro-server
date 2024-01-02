
use std::io::{Cursor, Read, BufReader};
use std::fmt::Debug;
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::collections::BTreeMap;
use std::path::Path;

const MIN_PACKET_SIZE: usize = 2;

#[derive(Debug)]
pub enum PacketLen {
	Fixed(i16),
	Variable,
}

#[derive(Debug)]
pub struct RawPacket {
	pub packet_id: u16,
	pub length: usize,
	buffer: Vec<u8>,
}

impl RawPacket {
	pub fn parse<P: Packet>(&self) -> Option<P> {
		<P>::deserialize(&self.buffer[..self.length])
	}
}

#[derive(Debug)]
pub struct PacketParser {
	length_table: BTreeMap<u16, (String, PacketLen)>,
}

impl PacketParser {
	pub fn new(path: &str) -> Self {
		let mut length_table = BTreeMap::new();
		
		let file = File::open(Path::new(path)).unwrap();
		let mut reader = BufReader::new(file);

		let mut buf = String::new();
		reader.read_to_string(&mut buf).unwrap();

		let rows = buf
			.split("\n")
			.filter(|line| line.trim().len() > 0 && !line.starts_with("#"));

		for line in rows {
			let sanitized = line
				.replace("\r", "");

			let row: Vec<&str> = sanitized
				.split("\t")
				.filter(|line| line.trim().len() > 0)
				.collect();

			if row.len() != 3 {
				continue;
			} else if row[0].len() < 3 {
				continue;
			}

			let id = u16::from_str_radix(&row[0][2..], 16).ok();
			let len = match i16::from_str_radix(&row[1], 10).ok() {
				Some(x) if x >= 2 => Some(PacketLen::Fixed(x)),
				Some(x) if x < 0 => Some(PacketLen::Variable),
				Some(_) => None,
				None => None
			};
			let name = row[2];

			if id.is_none() || len.is_none() {
				println!("Could not parse row of packet length file '{}': ({})", path, row.join(" | "));
				continue;
			}


			length_table.insert(id.unwrap(), (name.to_string(), len.unwrap()));
		}

		println!("Finished loading packet lengths from '{}'. Found {} valid packets.", path, length_table.len());
		
		Self {
			length_table
		}
	}

	pub fn extract_packets(&self, buf: &[u8]) -> Vec<RawPacket> {
		let mut cur = Cursor::new(buf);
		let mut packets = Vec::new();

		if buf.len() < MIN_PACKET_SIZE {
			println!("Buffer is smaller than the minimum packet size.");
			return packets;
		}

		while buf.len() > cur.position() as usize && MIN_PACKET_SIZE <= (buf.len() - cur.position() as usize) {
			let packet_id: u16;
			let start = cur.position() as usize;
			if let Some(pid) = cur.read_u16::<LittleEndian>().ok() {
				packet_id = pid;
			} else {
				break;
			}

			let length = match self.length_table.get(&packet_id) {
				Some((_, PacketLen::Fixed(len))) => {
					cur.set_position((start + *len as usize) as u64);
					*len
				},
				Some((_, PacketLen::Variable)) => {
					let len = cur.read_i16::<LittleEndian>().unwrap();
					cur.set_position((start + len as usize) as u64);
					len
				},
				None => {
					println!("Couldn't prepare unknown packet with id {:#06X}", packet_id);
					break;
				}
			} as usize;

			if length == 0 {
				println!("Received packet {:#06X} with zero bytes. Discarding received bytes...", packet_id);
				break;
			}
			
			if buf.len() < start + length {
				println!("Packet length is bigger than buffer! Packet id {:#06X} (Length: {}) | Buffer length: {}", packet_id, length, buf.len() - start);
				break;
			}

			println!("Adding packet to queue");
			packets.push(RawPacket {
				packet_id,
				length,
				buffer: buf[start..(start + length)].to_vec(),
			});
		}

		packets
	}
}

pub trait Packet: Default + Debug + Sized {
	fn new() -> Self;

	fn serialize(&self) -> Option<Vec<u8>>;

	fn deserialize(cursor: &[u8]) -> Option<Self>;

	fn has_valid_length(&self, length: usize) -> bool;

	fn len(&self) -> usize;
}

pub trait PacketFragment: Default + Debug + Sized {
	fn serialize(&self) -> Option<Vec<u8>>;

	fn deserialize(cursor: &mut std::io::Cursor<&[u8]>) -> Option<Self>;

	fn get_base_len() -> usize;
}

extern crate packet_derive;
pub use packet_derive::{Packet, PacketFragment};