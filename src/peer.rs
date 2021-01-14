use bitvec::{bitvec, order::Msb0, prelude::BitVec};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{manager::Block, Result};
use crate::{manager::Command, message::Msg};

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
enum ChokeState {
    Unchoked,
    Choked,
}

#[derive(Debug)]
enum InterestState {
    Interested,
    NotInterested,
}

#[derive(Debug)]
pub struct Peer {
    ip: String,
    port: u16,
    peer_id: Vec<u8>,
    // if we have choked the peer
    client_state: ChokeState,
    // if we are interested in the peer
    client_interest: InterestState,
    // if it has choked the client
    peer_state: ChokeState,
    // if the peer is interested in the client
    peer_interest: InterestState,
    transmitter: UnboundedSender<Command>,
}

impl Peer {
    pub fn new(
        ip: String,
        port: u16,
        peer_id: Vec<u8>,
        transmitter: UnboundedSender<Command>,
    ) -> Self {
        Self {
            ip,
            port,
            peer_id,
            client_state: ChokeState::Unchoked,
            client_interest: InterestState::Interested,
            peer_state: ChokeState::Choked,
            peer_interest: InterestState::NotInterested,
            transmitter,
        }
    }

    // TODO
    /// Perform integrity check on handshake
    /// Parse all the frames and perform appropriate action
    /// use Bytes and tokio
    /// read first four char and find length using it.
    /// create a vec buffer using the message length and read using read_exact
    pub async fn connect(&mut self, info_hash: &Vec<u8>, client_peer_id: &Vec<u8>) -> Result<()> {
        //let timeout = std::time::Duration::new(20, 0);
        let ip = format!("{}:{}", self.ip, self.port);
        //println!("IP-{} ", ip);

        // send handshake
        let handshake = Handshake::new(info_hash, client_peer_id);
        let handshake = handshake.generate_handshake();
        //println!("Sending handshake:- {}", handshake.len());
        let mut stream = TcpStream::connect(ip).await?;
        stream.write(&handshake).await?;

        // receive handshake
        let mut received_handshake = [0; 68];
        stream.read_exact(&mut received_handshake).await?;

        //println!("{:x?}", &received_handshake.to_vec());

        // integrity check
        let a = &received_handshake[28..48].to_vec() == info_hash;
        //println!("Info Check:- {}", a);

        let a = &received_handshake[48..].to_vec() == &self.peer_id;
        //println!("Peer id check:- {}", a);

        loop {
            let mut buffer = [0; 4];
            stream.read_exact(&mut buffer).await?;

            //println!("Message Length:- {:x?}", &buffer);
            let payload_length = u32::from_be_bytes(buffer);
            // keep alive message
            if payload_length == 0 {
                continue;
            }
            let mut buffer = vec![0; (payload_length) as usize];
            stream.read_exact(&mut buffer).await?;

            let msg = Msg::parse(buffer)?;
            //println!("{:x?}", msg);
            match msg {
                Msg::Bitfield(bitfield) => {
                    //todo might not need to clone peer id here
                    let peer_id = self.peer_id.clone();
                    self.transmitter
                        .send(Command::BitfieldRecieved { peer_id, bitfield })?;
                    // set current peer's bifield
                    //self.bitfield = bitfield;
                    // send interested msg
                    stream.write(&Msg::Interested.get_message()).await?;
                }
                Msg::Unchoke => {
                    self.peer_state = ChokeState::Unchoked;
                    let (tx, rx) = oneshot::channel::<Command>();
                    self.transmitter.send(Command::PickInitialPieces {
                        peer_id: self.peer_id.clone(),
                        transmitter: tx,
                    })?;
                    match rx.await? {
                        Command::SelectedInitialPieces(blocks) => {
                            // filter out None and convert rest to Request messages
                            let req_blocks: Vec<Msg> = blocks
                                .into_iter()
                                .filter_map(|block| block)
                                .map(|block| Msg::Request {
                                    index: block.piece_index,
                                    length: block.length,
                                    begin: block.begin,
                                })
                                .collect();

                            for req in req_blocks {
                                stream.write(&req.get_message()).await?;
                            }
                        }
                        Command::NoPiece => {
                            Err("No piece left to pick")?;
                        }
                        _ => {}
                    }
                    // send request msg
                    /* stream
                    .write(&[0, 0, 0, 13, 6, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 64, 0])
                    .await?; */
                }

                Msg::Choke => {
                    self.peer_state = ChokeState::Choked;
                }
                Msg::Interested => {
                    self.peer_interest = InterestState::Interested;
                }
                Msg::NotInterested => {
                    self.peer_interest = InterestState::NotInterested;
                }
                Msg::Have(piece_index) => {
                    self.transmitter.send(Command::HavePiece {
                        peer_id: self.peer_id.clone(),
                        piece_index: piece_index as usize,
                    })?;
                }
                Msg::Request {
                    index: _,
                    begin: _,
                    length: _,
                } => {
                    //ignore
                }
                Msg::Piece {
                    index: _,
                    begin: _,
                    block: _,
                } => {
                    // write the block
                    println!("Got piece from peer");
                    let (tx, rx) = oneshot::channel::<Command>();
                    self.transmitter.send(Command::PickPiece {
                        peer_id: self.peer_id.clone(),
                        transmitter: tx,
                    })?;

                    match rx.await? {
                        Command::SelectedPiece(block) => {
                            let req_block = Msg::Request {
                                index: block.piece_index,
                                length: block.length,
                                begin: block.begin,
                            };

                            stream.write(&req_block.get_message()).await?;
                        }
                        Command::NoPiece => {
                            Err("No piece left to pick")?;
                        }
                        _ => {}
                    }
                }
                Msg::Cancel {
                    index: _,
                    begin: _,
                    length: _,
                } => {
                    //ignore
                }
            }
        }
        Ok(())
    }
}
