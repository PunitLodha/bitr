use bitvec::{order::Msb0, prelude::BitVec};

use crate::Result;

#[derive(Debug)]
pub enum Msg {
    /// choke: <len=0001><id=0>
    Choke,
    /// unchoke: <len=0001><id=1>
    Unchoke,
    /// interested: <len=0001><id=2>
    Interested,
    /// not interested: <len=0001><id=3>
    NotInterested,
    /// have: <len=0005><id=4><piece index>
    Have(u32),
    /// bitfield: <len=0001+X><id=5><bitfield>
    Bitfield(BitVec<Msb0, u8>),
    /// request: <len=0013><id=6><index><begin><length>
    Request { index: u32, begin: u32, length: u32 },
    /// piece: <len=0009+X><id=7><index><begin><block>
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    /// cancel: <len=0013><id=8><index><begin><length>
    Cancel { index: u32, begin: u32, length: u32 },
}

impl Msg {
    pub fn parse(payload: Vec<u8>) -> Result<Msg> {
        let id = payload[0];
        let msg = match id {
            0 => Msg::Choke,
            1 => Msg::Unchoke,
            2 => Msg::Interested,
            3 => Msg::NotInterested,
            4 => {
                let index = u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]);
                Msg::Have(index)
            }
            5 => {
                let payload = payload[1..].to_vec();
                let bv = BitVec::<Msb0, u8>::from_vec(payload);
                Msg::Bitfield(bv)
            }
            6 => Msg::Request {
                index: u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]),
                begin: u32::from_be_bytes([payload[5], payload[6], payload[7], payload[8]]),
                length: u32::from_be_bytes([payload[9], payload[10], payload[11], payload[12]]),
            },
            7 => Msg::Piece {
                index: u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]),
                begin: u32::from_be_bytes([payload[5], payload[6], payload[7], payload[8]]),
                block: payload[9..].to_vec(),
            },
            8 => Msg::Cancel {
                index: u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]),
                begin: u32::from_be_bytes([payload[5], payload[6], payload[7], payload[8]]),
                length: u32::from_be_bytes([payload[9], payload[10], payload[11], payload[12]]),
            },
            _ => Err("Message ID is invalid")?,
        };
        Ok(msg)
    }
    /// Consume self and return a buffer with the respective message
    pub fn get_message(self) -> Vec<u8> {
        let mut message_buffer = vec![];
        match self {
            // choke: <len=0001><id=0>
            Msg::Choke => {
                message_buffer.extend_from_slice(&[0, 0, 0, 1, 0]);
            }
            // unchoke: <len=0001><id=1>
            Msg::Unchoke => {
                message_buffer.extend_from_slice(&[0, 0, 0, 1, 1]);
            }
            // interested: <len=0001><id=2>
            Msg::Interested => {
                message_buffer.extend_from_slice(&[0, 0, 0, 1, 2]);
            }
            // not interested: <len=0001><id=3>
            Msg::NotInterested => {
                message_buffer.extend_from_slice(&[0, 0, 0, 1, 3]);
            }
            // have: <len=0005><id=4><piece index>
            Msg::Have(index) => {
                message_buffer.extend_from_slice(&[0, 0, 0, 5, 4]);
                message_buffer.extend_from_slice(&(index.to_be_bytes()));
            }
            // bitfield: <len=0001+X><id=5><bitfield>
            Msg::Bitfield(bitfield) => {
                let bitfield = bitfield.into_vec();
                let message_len: u32 = 1 + bitfield.len() as u32;
                message_buffer.extend_from_slice(&(message_len.to_be_bytes()));
                message_buffer.extend_from_slice(&[5]);

                message_buffer.extend_from_slice(&(bitfield));
            }
            // request: <len=0013><id=6><index><begin><length>
            Msg::Request {
                index,
                begin,
                length,
            } => {
                message_buffer.extend_from_slice(&[0, 0, 0, 13, 6]);

                message_buffer.extend_from_slice(&(index.to_be_bytes()));
                message_buffer.extend_from_slice(&(begin.to_be_bytes()));
                message_buffer.extend_from_slice(&(length.to_be_bytes()));
            }
            // piece: <len=0009+X><id=7><index><begin><block>
            Msg::Piece {
                index,
                begin,
                block,
            } => {
                let message_len: u32 = 9 + block.len() as u32;
                message_buffer.extend_from_slice(&(message_len.to_be_bytes()));
                message_buffer.extend_from_slice(&[7]);

                message_buffer.extend_from_slice(&(index.to_be_bytes()));
                message_buffer.extend_from_slice(&(begin.to_be_bytes()));
                message_buffer.extend_from_slice(&block);
            }
            // cancel: <len=0013><id=8><index><begin><length>
            Msg::Cancel {
                index,
                begin,
                length,
            } => {
                message_buffer.extend_from_slice(&[0, 0, 0, 13, 8]);

                message_buffer.extend_from_slice(&(index.to_be_bytes()));
                message_buffer.extend_from_slice(&(begin.to_be_bytes()));
                message_buffer.extend_from_slice(&(length.to_be_bytes()));
            }
        }
        message_buffer
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use bitvec::bitvec;

    use super::*;
    #[test]
    fn test_choke_msg() -> Result<()> {
        let msg = Msg::Choke.get_message();
        assert_eq!(msg, &[0, 0, 0, 1, 0]);
        Ok(())
    }
    #[test]
    fn test_unchoke_msg() -> Result<()> {
        let msg = Msg::Unchoke.get_message();
        assert_eq!(msg, &[0, 0, 0, 1, 1]);
        Ok(())
    }
    #[test]
    fn test_interested_msg() -> Result<()> {
        let msg = Msg::Interested.get_message();
        assert_eq!(msg, &[0, 0, 0, 1, 2]);
        Ok(())
    }
    #[test]
    fn test_not_interested_msg() -> Result<()> {
        let msg = Msg::NotInterested.get_message();
        assert_eq!(msg, &[0, 0, 0, 1, 3]);
        Ok(())
    }
    #[test]
    fn test_have_msg() -> Result<()> {
        let msg = Msg::Have(10).get_message();
        assert_eq!(msg, &[0, 0, 0, 5, 4, 0, 0, 0, 10]);
        Ok(())
    }
    #[test]
    fn test_bitfield_msg() -> Result<()> {
        let bitfield = bitvec![Msb0,u8;1, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0, 1];
        let msg = Msg::Bitfield(bitfield).get_message();
        assert_eq!(msg, &[0, 0, 0, 3, 5, 213, 81]);
        Ok(())
    }
    #[test]
    fn test_request_msg() -> Result<()> {
        let msg = Msg::Request {
            index: 309,
            begin: 0,
            length: 16384,
        }
        .get_message();
        assert_eq!(msg, &[0, 0, 0, 13, 6, 0, 0, 1, 53, 0, 0, 0, 0, 0, 0, 64, 0]);
        Ok(())
    }
    #[test]
    fn test_piece_msg() -> Result<()> {
        let msg = Msg::Piece {
            index: 442,
            begin: 23,
            block: vec![0, 125, 39, 84, 64],
        }
        .get_message();
        assert_eq!(
            msg,
            &[0, 0, 0, 14, 7, 0, 0, 1, 186, 0, 0, 0, 23, 0, 125, 39, 84, 64]
        );
        Ok(())
    }
    #[test]
    fn test_cancel_msg() -> Result<()> {
        let msg = Msg::Cancel {
            index: 10000,
            begin: 1600,
            length: 16384,
        }
        .get_message();
        assert_eq!(
            msg,
            &[0, 0, 0, 13, 8, 0, 0, 39, 16, 0, 0, 6, 64, 0, 0, 64, 0]
        );
        Ok(())
    }
}
