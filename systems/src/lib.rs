use packet::RawPacket;

use network::PlayerSession;

pub mod auth;

#[derive(Debug)]
pub enum SystemResult {
  Processed,
  NotProcessed
}

pub type System = fn(session: &mut PlayerSession, packet: &RawPacket) -> SystemResult;