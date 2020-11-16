use std::fs;

extern crate serde;
extern crate serde_bencode;
#[macro_use]
extern crate serde_derive;
extern crate serde_bytes;

use serde_bencode::de;
use serde_bytes::ByteBuf;

#[derive(Debug, Deserialize)]
struct Info {
    name: String,
    pieces: ByteBuf,
    #[serde(rename = "piece length")]
    piece_length: i64,
    #[serde(default)]
    length: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct Torrent {
    info: Info,
    #[serde(default)]
    announce: Option<String>, // can be replaced by announce list
    #[serde(default)]
    #[serde(rename = "creation date")]
    creation_date: Option<i64>,
    comment: Option<String>,
    #[serde(default)]
    #[serde(rename = "created by")]
    created_by: Option<String>,
}

fn render_torrent(torrent: &Torrent) {
    println!("name:\t\t{}", torrent.info.name);
    println!("length:\t\t{:?}", torrent.info.length);
    println!("piece length:\t{:?}", torrent.info.piece_length);
    println!("announce:\t{:?}", torrent.announce);
    println!("created by:\t{:?}", torrent.created_by);
    println!("creation date:\t{:?}", torrent.creation_date);
    println!("comment:\t{:?}", torrent.comment);
}

fn main() {
    let contents = fs::read("../kubuntu-20.10-desktop-amd64.iso.torrent")
        .expect("Something went wrong while reading the file");
    match de::from_bytes::<Torrent>(&contents) {
        Ok(t) => render_torrent(&t),
        Err(e) => println!("ERROR: {:?}", e),
    }
}
