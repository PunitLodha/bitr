use std::path::Path;

extern crate serde;
extern crate serde_bencode;
#[macro_use]
extern crate serde_derive;
extern crate serde_bytes;

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
    let url = torrent.generate_tracker_url();
    println!("{}", url.as_str());

    tracker::send_tracker_request(url);
}
