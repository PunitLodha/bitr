use reqwest::Url;
use serde_bencode::de;
use serde_bytes::ByteBuf;
use std::fmt;

use crate::Result;

#[derive(Debug, Deserialize)]
pub struct TrackerPeer {
    pub ip: String,
    pub port: u16,
    #[serde(rename = "peer id")]
    pub peer_id: ByteBuf,
}
#[derive(Debug, Deserialize)]
pub struct TrackerResponse {
    pub peers: Vec<TrackerPeer>,
    complete: i64,
    incomplete: i64,
    interval: i64,
    #[serde(rename = "tracker id")]
    tracker_id: Option<String>,
    #[serde(rename = "warning message")]
    warning_message: Option<String>,
    #[serde(rename = "min interval")]
    min_interval: Option<i64>,
}

impl fmt::Display for TrackerResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for peer in &self.peers {
            writeln!(f, "peer:\t\t{}:{}", peer.ip, peer.port)?;
        }
        writeln!(f, "complete:\t\t{:?}", self.complete)?;
        writeln!(f, "incomplete:\t\t{:?}", self.incomplete)?;
        writeln!(f, "interval:\t\t{:?}", self.interval)?;
        writeln!(f, "min interval:\t\t{:?}", self.min_interval)?;
        writeln!(f, "tracker id:\t\t{:?}", self.tracker_id)
    }
}

pub fn send_tracker_request(url: Url) -> Result<TrackerResponse> {
    match url.scheme() {
        "https" => handle_https_scheme(url),
        "udp" => todo!("Support for UDP trackers"),
        _ => Err("URL scheme not supported")?,
    }
}

fn handle_https_scheme(url: Url) -> Result<TrackerResponse> {
    // send get request to tracker
    let mut response = reqwest::blocking::get(url)?;
    //println!("{:?}", response.text().unwrap());
    let mut buf: Vec<u8> = vec![];
    response.copy_to(&mut buf).unwrap();

    // deserialize response to TrackerResponse
    let tracker_res = de::from_bytes::<TrackerResponse>(&buf)?;
    println!("{}", tracker_res);
    Ok(tracker_res)
}
