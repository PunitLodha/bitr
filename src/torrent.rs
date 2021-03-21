use reqwest::Url;
use ring::digest;
use serde_bencode::{de, ser};
use serde_bytes::ByteBuf;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use crate::utils::bytes_to_string_with_encoding;
use crate::Result;

#[derive(Debug, Deserialize, Serialize)]
pub struct Info {
    pub name: String,
    pub pieces: ByteBuf,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    #[serde(default)]
    pub length: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Torrent {
    pub info: Info,
    #[serde(default)]
    announce: Option<String>,
    #[serde(default)]
    #[serde(rename = "creation date")]
    creation_date: Option<i64>,
    comment: Option<String>,
    #[serde(default)]
    #[serde(rename = "created by")]
    created_by: Option<String>,
    #[serde(skip)]
    pub info_hash: Vec<u8>,
}

impl fmt::Display for Torrent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "name:\t\t{}", self.info.name)?;
        writeln!(f, "length:\t\t{:?}", self.info.length)?;
        writeln!(f, "piece length:\t{:?}", self.info.piece_length)?;
        writeln!(f, "announce:\t{:?}", self.announce)?;
        writeln!(f, "created by:\t{:?}", self.created_by)?;
        writeln!(f, "creation date:\t{:?}", self.creation_date)?;
        writeln!(f, "comment:\t{:?}", self.comment)?;
        writeln!(f, "info hash:\t{:?}", self.info_hash)
    }
}

impl Torrent {
    pub fn new(path: &PathBuf) -> Result<Torrent> {
        let contents = fs::read(path)?;

        // deserialize the file to torrent struct
        let torrent = de::from_bytes::<Torrent>(&contents)?;

        let info_hash = torrent.generate_info_hash()?;
        Ok(Torrent {
            info_hash,
            ..torrent
        })
    }

    fn generate_info_hash(&self) -> Result<Vec<u8>> {
        let ser_info = ser::to_bytes(&self.info)?;
        let x = digest::digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &ser_info);
        let info_hash = x.as_ref();
        let mut vec: Vec<u8> = Vec::new();
        vec.extend_from_slice(info_hash);
        Ok(vec)
    }

    pub fn generate_tracker_url(&self, peer_id: &Vec<u8>) -> Result<Url> {
        // bittorrent port
        const PORT: i32 = 6881;
        let length = self.info.length.ok_or("File length is missing")?;
        let info_hash = bytes_to_string_with_encoding(&self.info_hash)?;
        let peer_id = bytes_to_string_with_encoding(peer_id)?;

        // parse the url using reqwest
        let announce_url = self.announce.as_ref().ok_or("Announce url missing")?;

        let url = format!(
            "{url}?info_hash={info_hash}&peer_id={peer_id}",
            url = announce_url,
            info_hash = info_hash,
            peer_id = peer_id
        );

        let mut url = Url::parse(&url)?;

        url.query_pairs_mut()
            .append_pair("port", &PORT.to_string())
            .append_pair("uploaded", "0")
            .append_pair("downloaded", "0")
            .append_pair("left", &length.to_string());
        Ok(url)
    }
}
