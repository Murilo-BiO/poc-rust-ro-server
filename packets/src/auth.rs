use packet::{Packet, PacketFragment};

#[derive(Debug)]
#[repr(u16)]
pub enum PacketId {
    // Received
    CaLogin = 0x0064,
    CaSsoLoginReq = 0x0825,
    
    // Transmitted
    AcAcceptLogin = 0x0069,
    AcAcceptLogin2 = 0x0AC4,
    AcRefuseLogin = 0x006A,
}

#[derive(Debug, Default, Packet)]
#[packet(id = "CaSsoLoginReq")]
pub struct PacketCaSsoLoginReq {
	pub packet_id: u16,
	pub packet_len: i16,
	pub version: u32,
	pub client_type: u8,
	pub id: [u8; 24],
	pub password: [u8; 27],
	pub mac_address: [i8; 17],
	pub ip: [u8; 15],
	pub t1: Vec<u8>
}

#[derive(Debug, Default, Packet)]
#[packet(id = "CaLogin")]
pub struct PacketCaLogin {
	pub packet_id: u16,
    pub version: u32,
    pub username: [u8; 24],
    pub password: [u8; 24],
    pub client_type: u8,
}

#[derive(Debug, Default, Packet)]
#[packet(id = "AcAcceptLogin2")]
pub struct PacketAcAcceptLogin2 {
    pub packet_id: u16,
    pub packet_len: i16,
    pub auth_code: i32,
    pub aid: u32,
    pub user_level: u32,
    pub last_login_ip: u32,
    pub last_login_time: [u8; 26],
    pub sex: u8,
    pub twitter_auth_token: [u8; 16],
    pub twitter_flag: u8,
    pub char_server_list: Vec<CharServerList>,
}

// Helper Structs
#[derive(Debug, PacketFragment)]
pub struct CharServerList {
    pub ip: u32,
    pub port: i16,
    pub name: [u8; 20],
    pub usercount: u16,
    pub is_new: u16,
    pub server_type: u16,
    pub unknown2: [u8; 128],
}

impl Default for CharServerList {
    fn default() -> Self {
        Self {
            ip: 0,
            port: 0,
            name: [0; 20],
            usercount: 0,
            is_new: 0,
            server_type: 0,
            unknown2: [0; 128],
        }
    }
}