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
    let x: Vec<&[u8]> = pieces.chunks_exact(20).collect();
    println!("{}", x.len());
    let url = torrent.generate_tracker_url(&client.peer_id)?;
    println!("{}", url.as_str());

    let res = tracker::send_tracker_request(url)?;

    // todo change this
    let peer = res.peers.iter().next().unwrap();
    peer::connect_to_peer(peer, &torrent.info_hash, &client.peer_id)?;

    Ok(())
    /* for peer in res.peers.iter() {
        peer::connect_to_peer(peer, &torrent.info_hash, &peer_id);
    } */
}
