use std::io::{Read, Write};
use std::net::TcpStream;

use crate::tracker::TrackerPeer;
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
            info_hash: info_hash,
            peer_id: peer_id,
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

enum PeerState {
    Interested,
    Choked,
}

enum Msg {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield,
    Request,
    Piece,
    Cancel,
}

impl Msg {
    fn new(id: u8) -> Result<Msg> {
        let msg = match id {
            0 => Msg::Choke,
            1 => Msg::Unchoke,
            2 => Msg::Interested,
            3 => Msg::NotInterested,
            4 => Msg::Have,
            5 => Msg::Bitfield,
            6 => Msg::Request,
            7 => Msg::Piece,
            8 => Msg::Cancel,
            _ => Err("Message ID is invalid")?,
        };
        Ok(msg)
    }
}

struct Peer<'a> {
    /* ip: String,
    port: u16,
    peer_id: Vec<u8>, */
    tracker_peer: &'a TrackerPeer,
    bit_field: Option<Vec<u8>>,
    // this is to identify if we are interested in the peer or if we have choked the peer
    our_state: PeerState,
    // this is to identify if the peer is interested in the client or if it has choked the client
    peer_state: PeerState,
}
struct Manager {}

pub fn connect_to_peer(peer: &TrackerPeer, info_hash: &Vec<u8>, peer_id: &Vec<u8>) -> Result<()> {
    let timeout = std::time::Duration::new(20, 0);
    let ip = format!("{}:{}", &peer.ip, &peer.port);
    println!("IP-{} ", ip);

    let handshake = Handshake::new(&info_hash, &peer_id);
    let handshake = handshake.generate_handshake();
    println!("Sending handshake:- {}", handshake.len());

    let mut stream = TcpStream::connect_timeout(&ip.parse()?, timeout)?;
    stream.write(&handshake)?;
    loop {
        let mut buffer = [0; u16::MAX as usize];
        match stream.read(&mut buffer) {
            Ok(n) => {
                if n > 0 {
                    println!("{:x?}", &buffer[0..n])
                }
            }
            Err(e) => println!("Error: {}", e),
        }
    }

    /*   // Wrap the stream in a BufReader, so we can use the BufRead methods
       let mut reader = std::io::BufReader::new(&mut stream);

       // Read current current data in the TcpStream
       let mut received_handshake: [u8; 68] = [0; 68];
       reader.read_exact(&mut received_handshake)?;
       println!("{:x?}", &received_handshake.to_vec());

       let a = &received_handshake[28..48].to_vec() == info_hash;
       println!("{}", a);

       // Read current current data in the TcpStream
       let received: Vec<u8> = reader.fill_buf()?.to_vec();

       println!("{:x?}", &received);
       reader.consume(received.len());

       // todo first read 4 bytes, then mess_len + 1 bytes

       let mut message_length: [u8; 4] = [0; 4];

       message_length[..4].copy_from_slice(&received[0..4]);

       let value = i32::from_be_bytes(message_length);

       println!("{}   {}", value, received.len());
    */
    Ok(())
}
