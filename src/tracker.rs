use reqwest::Url;
use serde_bencode::de;
use serde_bytes::ByteBuf;
use std::fmt;

#[derive(Debug, Deserialize)]
struct Peer {
    ip: String,
    port: i64,
    #[serde(rename = "peer id")]
    peer_id: ByteBuf,
}
#[derive(Debug, Deserialize)]
struct TrackerResponse {
    peers: Vec<Peer>,
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

pub fn send_tracker_request(url: Url) {
    // send get request to tracker
    let mut response = reqwest::blocking::get(url).unwrap();
    //println!("{:?}", response.text().unwrap());
    let mut buf: Vec<u8> = vec![];
    response.copy_to(&mut buf).unwrap();

    // deserialize response to TrackerResponse
    let tracker_res = de::from_bytes::<TrackerResponse>(&buf).expect("Err");
    println!("{}", tracker_res);

    /* match announce_url.scheme() {
        "https" => {
            let info = torrent.info;
            let ser_info = ser::to_bytes(&info).unwrap();
            let mut hasher = Sha1::new();
            hasher.update(ser_info);
            let res = hasher.finalize();
            println!("{:x}", res);
        }
        "udp" => todo!("Support for UDP trackers"),
        _ => panic!("Url scheme not supported"),
    }; */
}
