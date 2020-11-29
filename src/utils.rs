use ring::{rand::SecureRandom, rand::SystemRandom};
use std::fmt::Write;

use crate::Result;

pub fn bytes_to_string_with_encoding(bytes: &[u8]) -> Result<String> {
    let mut s = String::with_capacity(40);
    for byte in bytes {
        write!(s, "%{:02X}", byte).unwrap();
    }
    Ok(s)
}

pub fn generate_peer_id() -> Result<Vec<u8>> {
    let generator = SystemRandom::new();
    let mut bytes: Vec<u8> = vec![0; 20];
    generator.fill(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_num() -> Result<()> {
        let s = bytes_to_string_with_encoding(&[19])?;
        assert_eq!(s, "%13");
        Ok(())
    }
}
