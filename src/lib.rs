use std::convert::TryInto;
use std::error::Error;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[macro_use]
extern crate serde_derive;

mod manager;
mod message;
mod peer;
mod torrent;
mod tracker;
mod utils;

use manager::Command;
use peer::Peer;
use torrent::Torrent;

// create an alias for the result type
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

struct Client {
    path: PathBuf,
    peer_id: Vec<u8>,
}

impl Client {
    fn new(file_path: String) -> Result<Client> {
        // path of the torrent file
        let path = PathBuf::from(file_path);
        // generate the peer id
        let peer_id = utils::generate_peer_id()?;
        Ok(Client { path, peer_id })
    }
}

pub async fn run() -> Result<()> {
    let file_path = std::env::args()
        .nth(1)
        .ok_or("path to torrent file is missing\nUsage: bitr <path to torrent file>")?;
    let client = Client::new(file_path)?;

    let torrent = Torrent::new(&client.path)?;
    println!("{}", &torrent);

    let pieces = torrent.info.pieces.to_vec();
    let piece_hashes: Vec<[u8; 20]> = pieces
        .chunks_exact(20)
        // try to remove this unwrap
        .map(|chunk| chunk.try_into().unwrap())
        .collect();
    let no_of_pieces = piece_hashes.len();
    println!("{}", no_of_pieces);

    let piece_length = torrent.info.piece_length;

    let url = torrent.generate_tracker_url(&client.peer_id)?;
    println!("{}", url.as_str());

    let res = tracker::send_tracker_request(url)?;

    let info_hash = &torrent.info_hash;

    let (send_to_manager, mut recieve_from_peers) = mpsc::unbounded_channel::<Command>();

    let peer_list = res
        .peers
        .into_iter()
        .map(|tracker_peer| {
            Peer::new(
                tracker_peer.ip,
                tracker_peer.port,
                tracker_peer.peer_id.to_vec(),
                send_to_manager.clone(),
            )
        })
        .collect::<Vec<Peer>>();

    /* let manager_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        let mut manager =
            manager::Manager::new(no_of_pieces as u32, piece_hashes, piece_length as u32);

        while let Some(cmd) = recieve_from_peers.recv().await {
            match cmd {
                Command::BitfieldRecieved { peer_id, bitfield } => {
                    manager.piece_picker.register_bitfield(peer_id, bitfield);
                    println!("Recieved bitfield from peer");
                }
                Command::PickInitialPieces {
                    peer_id,
                    transmitter,
                } => {
                    let blocks = manager.piece_picker.pick_intial_pieces(&peer_id);
                    match blocks {
                        Some(blks) => {
                            if let Err(_) = transmitter.send(Command::SelectedInitialPieces(blks)) {
                                eprintln!("Receiver Dropped");
                            };
                        }
                        None => {
                            if let Err(_) = transmitter.send(Command::NoPiece) {
                                eprintln!("Receiver Dropped");
                            };
                        }
                    }
                }
                Command::PickPiece {
                    peer_id,
                    transmitter,
                } => {
                    let block = manager.piece_picker.pick_piece(&peer_id);
                    match block {
                        Some(blk) => {
                            if let Err(_) = transmitter.send(Command::SelectedPiece(blk)) {
                                eprintln!("Receiver Dropped");
                            };
                        }
                        None => {
                            if let Err(_) = transmitter.send(Command::NoPiece) {
                                eprintln!("Receiver Dropped");
                            };
                        }
                    }
                }
                Command::HavePiece {
                    peer_id,
                    piece_index,
                } => {
                    manager
                        .piece_picker
                        .increment_piece_availability(piece_index);
                    if let Some(bitfield) = manager.piece_picker.peer_bitfields.get_mut(&peer_id) {
                        *bitfield.get_mut(piece_index).unwrap() = true;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }); */

    let handles: Vec<JoinHandle<()>> = peer_list
        .into_iter()
        .map(|mut peer| {
            let info = info_hash.clone();
            let client_peer_id = client.peer_id.clone();
            tokio::spawn(async move {
                if let Err(e) = peer.connect(&info, &client_peer_id).await {
                    eprintln!("Some error occured:- {:?}", e);
                    eprintln!("Closing the connection");
                };
            })
        })
        .collect();

    let mut manager = manager::Manager::new(no_of_pieces as u32, piece_hashes, piece_length as u32);

    while let Some(cmd) = recieve_from_peers.recv().await {
        match cmd {
            Command::BitfieldRecieved { peer_id, bitfield } => {
                manager.piece_picker.register_bitfield(peer_id, bitfield);
                println!("Recieved bitfield from peer");
            }
            Command::PickInitialPieces {
                peer_id,
                transmitter,
            } => {
                let blocks = manager.piece_picker.pick_intial_pieces(&peer_id);
                match blocks {
                    Some(blks) => {
                        if let Err(_) = transmitter.send(Command::SelectedInitialPieces(blks)) {
                            eprintln!("Receiver Dropped");
                        };
                    }
                    None => {
                        if let Err(_) = transmitter.send(Command::NoPiece) {
                            eprintln!("Receiver Dropped");
                        };
                    }
                }
            }
            Command::PickPiece {
                peer_id,
                transmitter,
            } => {
                let block = manager.piece_picker.pick_piece(&peer_id);
                match block {
                    Some(blk) => {
                        if let Err(_) = transmitter.send(Command::SelectedPiece(blk)) {
                            eprintln!("Receiver Dropped");
                        };
                    }
                    None => {
                        if let Err(_) = transmitter.send(Command::NoPiece) {
                            eprintln!("Receiver Dropped");
                        };
                    }
                }
            }
            Command::HavePiece {
                peer_id,
                piece_index,
            } => {
                manager
                    .piece_picker
                    .increment_piece_availability(piece_index);
                if let Some(bitfield) = manager.piece_picker.peer_bitfields.get_mut(&peer_id) {
                    *bitfield.get_mut(piece_index).unwrap() = true;
                }
            }
            _ => {}
        }
    }

    /* let handles: Vec<JoinHandle<Result<()>>> = peer_list
           .into_iter()
           .map(|mut peer| {
               let info = info_hash.clone();
               let client_peer_id = client.peer_id.clone();
               tokio::spawn(async move {
                   if let Err(e) = peer.connect(&info, &client_peer_id).await {
                       eprintln!("Some error occured:- {:?}", e);
                       eprintln!("Closing the connection");
                   };
                   Ok(())
               })
           })
           .collect();
    */

    for handle in handles {
        handle.await?;
    }
    //manager_task.await??;

    Ok(())
}
