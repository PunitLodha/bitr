use ring::{rand::SecureRandom, rand::SystemRandom};
use std::fmt::Write;

pub fn bytes_to_string_with_encoding(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(40);
    for byte in bytes {
        write!(s, "%{:02X}", byte).unwrap();
    }
    s
}

pub fn generate_peer_id() -> Vec<u8> {
    let generator = SystemRandom::new();
    let mut bytes: Vec<u8> = vec![0; 20];
    generator.fill(&mut bytes).unwrap();
    bytes
}
