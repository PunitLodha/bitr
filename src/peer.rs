use bitvec::{bitvec, order::Msb0, prelude::BitVec};
use std::io::{Read, Write};
use std::net::TcpStream;

use crate::message::Msg;
use crate::Result;

struct Handshake<'a> {
    info_hash: &'a Vec<u8>,
    peer_id: &'a Vec<u8>,
    reserved_bytes: Vec<u8>,
    protocol: Vec<u8>,
    protocol_length: Vec<u8>,
}

impl<'a> Handshake<'a> {
    pub fn new(info_hash: &'a Vec<u8>, peer_id: &'a Vec<u8>) -> Self {
        let reserved_bytes = vec![0; 8];
        let protocol = b"BitTorrent protocol".to_vec();
        let protocol_length = vec![19];
        Handshake {
            info_hash,
            peer_id,
            reserved_bytes,
            protocol,
            protocol_length,
        }
    }
    pub fn generate_handshake(self) -> Vec<u8> {
        let Handshake {
            info_hash,
            peer_id,
            reserved_bytes,
            protocol,
            protocol_length,
        } = self;

        let handshake = [
            &protocol_length,
            &protocol,
            &reserved_bytes,
            info_hash.as_slice(),
            peer_id.as_slice(),
        ]
        .concat();
        handshake
    }
}

#[derive(Debug)]
enum PeerState {
    Interested,
    Choked,
}

#[derive(Debug)]
pub struct Peer {
    ip: String,
    port: u16,
    peer_id: Vec<u8>,
    bitfield: BitVec<Msb0, u8>,
    // this is to identify if we are interested in the peer or if we have choked the peer
    our_state: PeerState,
    // this is to identify if the peer is interested in the client or if it has choked the client
    peer_state: PeerState,
}

impl Peer {
    pub fn new(ip: String, port: u16, peer_id: Vec<u8>, no_of_pieces: usize) -> Self {
        Self {
            ip,
            port,
            peer_id,
            our_state: PeerState::Interested,
            peer_state: PeerState::Choked,
            bitfield: bitvec![Msb0,u8;0;no_of_pieces],
        }
    }

    // TODO
    /// Perform integrity check on handshake
    /// Parse all the frames and perform appropriate action
    /// use Bytes and tokio
    /// read first four char and find length using it.
    /// create a vec buffer using the message length and read using read_exact
    pub fn connect(&self, info_hash: &Vec<u8>) -> Result<()> {
        let timeout = std::time::Duration::new(20, 0);
        let ip = format!("{}:{}", self.ip, self.port);
        println!("IP-{} ", ip);

        // send handshake
        let handshake = Handshake::new(&info_hash, &self.peer_id);
        let handshake = handshake.generate_handshake();
        println!("Sending handshake:- {}", handshake.len());

        let mut stream = TcpStream::connect_timeout(&ip.parse()?, timeout)?;
        stream.write(&handshake)?;

        // receive handshake
        let mut received_handshake = [0; 68];
        stream.read_exact(&mut received_handshake)?;
        println!("{:x?}", &received_handshake.to_vec());

        // integrity check
        let a = &received_handshake[28..48].to_vec() == info_hash;
        println!("Info Check:- {}", a);

        let a = &received_handshake[48..].to_vec() == &self.peer_id;
        println!("Peer id check:- {}", a);

        loop {
            let mut buffer = [0; 4];
            stream.read_exact(&mut buffer)?;
            println!("Message Length:- {:x?}", &buffer);
            let payload_length = u32::from_be_bytes(buffer);
            // keep alive message
            if payload_length == 0 {
                continue;
            }
            let mut buffer = vec![0; (payload_length) as usize];
            stream.read_exact(&mut buffer)?;
            let msg = Msg::parse(buffer)?;
            println!("{:x?}", msg);
            match msg {
                Msg::Bitfield(_) => {
                    //todo increase piece availability for each piece in bitfield
                    // send interested msg
                    stream.write(&[0, 0, 0, 1, 2])?;
                }
                Msg::Unchoke => {
                    // send request msg
                    stream.write(&[0, 0, 0, 13, 6, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 64, 0])?;
                }

                Msg::Choke => {}
                Msg::Interested => {}
                Msg::NotInterested => {}
                Msg::Have(_) => {}
                Msg::Request {
                    index,
                    begin,
                    length,
                } => {}
                Msg::Piece {
                    index,
                    begin,
                    block,
                } => {}
                Msg::Cancel {
                    index,
                    begin,
                    length,
                } => {}
            }
        }
    }
}
