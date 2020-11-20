use std::path::Path;

#[macro_use]
extern crate serde_derive;

mod torrent;
mod tracker;
mod utils;

use crate::torrent::Torrent;

fn main() {
    // path of the torrent file
    // TODO  change this so that it can be taken as an argument
    let path = Path::new("../kubuntu-20.10-desktop-amd64.iso.torrent");

    let torrent = Torrent::new(&path);
    println!("{}", &torrent);

    let peer_id = utils::generate_peer_id();

    let url = torrent.generate_tracker_url(&peer_id);
    println!("{}", url.as_str());

    tracker::send_tracker_request(url);
}
