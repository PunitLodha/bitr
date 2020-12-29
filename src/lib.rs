use std::convert::TryInto;
use std::error::Error;
use std::path::PathBuf;

#[macro_use]
extern crate serde_derive;

mod manager;
mod message;
mod peer;
mod torrent;
mod tracker;
mod utils;

use peer::Peer;

use crate::torrent::Torrent;

// create an alias for the result type
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

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

pub fn run() -> Result<()> {
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

    // todo change this
    //let peer = res.peers.iter().next().unwrap();
    let peers = res
        .peers
        .into_iter()
        .map(|tracker_peer| {
            Peer::new(
                tracker_peer.ip,
                tracker_peer.port,
                tracker_peer.peer_id.to_vec(),
                no_of_pieces,
            )
        })
        .collect::<Vec<Peer>>();
    peers.iter().nth(1).unwrap().connect(&torrent.info_hash)?;
    //peer.connect_to_peer(peer, &torrent.info_hash, &client.peer_id)?;

    Ok(())
    /* for peer in res.peers.iter() {
        peer::connect_to_peer(peer, &torrent.info_hash, &peer_id);
    } */
}
