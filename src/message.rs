use crate::Result;

#[derive(Debug)]
pub enum Msg {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
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
                println!("{}", payload.len());
                Msg::Bitfield(payload)
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
}
