use std::fmt;
use std::io;
use std::net::UdpSocket;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MagicPacketError {
    #[error("invalid MAC address: {0}")]
    InvalidMac(String),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("network error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Clone, Debug)]
pub struct MagicPacket {
    mac: [u8; 6],
}

impl MagicPacket {
    pub fn from_str(s: &str) -> Result<Self, MagicPacketError> {
        let mac = parse_mac(s)?;
        Ok(Self { mac })
    }

    pub fn bytes(&self) -> [u8; 102] {
        let mut buf = [0xFFu8; 102];
        for i in 0..16 {
            let offset = 6 + i * 6;
            buf[offset..offset + 6].copy_from_slice(&self.mac);
        }
        buf
    }

    pub fn broadcast(&self, addr: &str) -> Result<(), MagicPacketError> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;
        socket.send_to(&self.bytes(), addr)?;
        Ok(())
    }
}

impl fmt::Display for MagicPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.mac[0], self.mac[1], self.mac[2], self.mac[3], self.mac[4], self.mac[5]
        )
    }
}

fn parse_mac(s: &str) -> Result<[u8; 6], MagicPacketError> {
    let parts: Vec<&str> = if s.contains(':') {
        s.split(':').collect()
    } else if s.contains('-') {
        s.split('-').collect()
    } else {
        return Err(MagicPacketError::InvalidMac(s.to_string()));
    };

    if parts.len() != 6 {
        return Err(MagicPacketError::InvalidMac(s.to_string()));
    }

    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        mac[i] = u8::from_str_radix(part, 16)
            .map_err(|_| MagicPacketError::InvalidMac(s.to_string()))?;
    }
    Ok(mac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_size() {
        let pkt = MagicPacket::from_str("aa:bb:cc:dd:ee:ff").unwrap();
        assert_eq!(pkt.bytes().len(), 102);
    }

    #[test]
    fn test_sync_stream() {
        let pkt = MagicPacket::from_str("00:11:22:33:44:55").unwrap();
        let bytes = pkt.bytes();
        for b in &bytes[..6] {
            assert_eq!(*b, 0xFF);
        }
    }

    #[test]
    fn test_mac_repeated_16_times() {
        let pkt = MagicPacket::from_str("aa:bb:cc:dd:ee:ff").unwrap();
        let bytes = pkt.bytes();
        let expected_mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        for i in 0..16 {
            let offset = 6 + i * 6;
            assert_eq!(&bytes[offset..offset + 6], &expected_mac);
        }
    }

    #[test]
    fn test_parse_mac_colon() {
        let pkt = MagicPacket::from_str("01:23:45:67:89:ab").unwrap();
        assert_eq!(pkt.mac, [0x01, 0x23, 0x45, 0x67, 0x89, 0xab]);
    }

    #[test]
    fn test_parse_mac_dash() {
        let pkt = MagicPacket::from_str("01-23-45-67-89-AB").unwrap();
        assert_eq!(pkt.mac, [0x01, 0x23, 0x45, 0x67, 0x89, 0xab]);
    }

    #[test]
    fn test_parse_mac_invalid() {
        assert!(MagicPacket::from_str("not-a-mac").is_err());
        assert!(MagicPacket::from_str("01:23:45:67:89").is_err());
        assert!(MagicPacket::from_str("01:23:45:67:89:ZZ").is_err());
    }

    #[test]
    fn test_display() {
        let pkt = MagicPacket::from_str("AA:BB:CC:DD:EE:FF").unwrap();
        assert_eq!(format!("{}", pkt), "aa:bb:cc:dd:ee:ff");
    }
}
