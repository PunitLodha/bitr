use std::fmt;
use std::fs;
use std::path::Path;

use reqwest::Url;
use ring::{digest, rand::SecureRandom, rand::SystemRandom};
use serde_bencode::{de, ser};
use serde_bytes::ByteBuf;

use crate::utils::bytes_to_string_with_encoding;

#[derive(Debug, Deserialize, Serialize)]
struct Info {
    name: String,
    pieces: ByteBuf,
    #[serde(rename = "piece length")]
    piece_length: i64,
    #[serde(default)]
    length: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct Torrent {
    info: Info,
    #[serde(default)]
    announce: Option<String>,
    #[serde(default)]
    #[serde(rename = "creation date")]
    creation_date: Option<i64>,
    comment: Option<String>,
    #[serde(default)]
    #[serde(rename = "created by")]
    created_by: Option<String>,
}

impl fmt::Display for Torrent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "name:\t\t{}", self.info.name)?;
        writeln!(f, "length:\t\t{:?}", self.info.length)?;
        writeln!(f, "piece length:\t{:?}", self.info.piece_length)?;
        writeln!(f, "announce:\t{:?}", self.announce)?;
        writeln!(f, "created by:\t{:?}", self.created_by)?;
        writeln!(f, "creation date:\t{:?}", self.creation_date)?;
        writeln!(f, "comment:\t{:?}", self.comment)
    }
}

impl Torrent {
    pub fn new(path: &Path) -> Self {
        let contents = fs::read(path).expect("Something went wrong while reading the file");

        // deserialize the file to torrent struct
        let torrent = de::from_bytes::<Torrent>(&contents).expect("Error parsing torrent");
        torrent
    }

    fn generate_info_hash(&self) -> String {
        let ser_info = ser::to_bytes(&self.info).unwrap();
        let x = digest::digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &ser_info);
        let info_hash = x.as_ref();
        let encoded_info_hash = bytes_to_string_with_encoding(&info_hash);
        encoded_info_hash
    }

    pub fn generate_tracker_url(&self) -> Url {
        // bittorrent port
        const PORT: i32 = 6881;
        let length = self.info.length;

        let info_hash = self.generate_info_hash();
        let peer_id = generate_peer_id();

        // parse the url using reqwest
        let announce_url = self.announce.as_ref().unwrap();

        let url = format!(
            "{url}?info_hash={info_hash}&peer_id={peer_id}",
            url = announce_url,
            info_hash = info_hash,
            peer_id = peer_id
        );

        let mut url = Url::parse(&url).unwrap();

        url.query_pairs_mut()
            .append_pair("port", &PORT.to_string())
            .append_pair("uploaded", "0")
            .append_pair("downloaded", "0")
            .append_pair("left", &length.unwrap().to_string());
        url
    }
}

fn generate_peer_id() -> String {
    let generator = SystemRandom::new();
    let mut bytes: Vec<u8> = vec![0; 20];
    generator.fill(&mut bytes).unwrap();
    let encoded_peer_id = bytes_to_string_with_encoding(&bytes);
    encoded_peer_id
}
