use std::error::Error;
use tokio::sync::mpsc;

#[macro_use]
extern crate serde_derive;

mod manager;
mod message;
mod peer;
mod torrent;
mod tracker;
mod utils;

use manager::{Command, Manager};

// create an alias for the result type
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub async fn run() -> Result<()> {
    // path to the torrent file
    let file_path = std::env::args()
        .nth(1)
        .ok_or("path to torrent file is missing\nUsage: bitr <path to torrent file>")?;

    let manager = Manager::new(file_path)?;
    // send request to tracker to get the list of peers
    let res = manager.send_tracker_request()?;

    // create mpsc channel for communication between piece picker and all peers
    let (send_to_manager, receive_from_peers) = mpsc::unbounded_channel::<Command>();
    // spawn a new tokio task for each peer
    let handles = manager.connect_to_peers(res, send_to_manager);

    let mut piece_picker = manager.spawn_piece_picker();
    // listen on mpsc channel for different commands from the peers
    piece_picker.listen_to_commands(receive_from_peers).await;

    for handle in handles {
        handle.await?;
    }

    Ok(())
}
