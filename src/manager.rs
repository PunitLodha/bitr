use std::collections::HashMap;
// TODO
/// Create a mpsc channel and clone(Arc + Mutex) its reciever passing it to all the peer task.
/// All jobs will be sent through this channel, and one by one recieved by the peer tasks
/// Also create a oneshot channel for getting back the data downloaded from the peers
struct Manager {}

struct PiecePicker {
    piece_map: Vec<PiecePos>,
    pieces: Vec<u32>,
    priority_boundaries: Vec<u32>,
    downloading: HashMap<u32, DownloadingPiece>,
}

struct PiecePos {
    peer_count: u32,
    state: PieceState,
    index: u32,
}

enum PieceState {
    Downloading,
    NotDownloading,
}

struct DownloadingPiece {
    index: u32,
    block_state: Vec<BlockState>,
}

enum BlockState {
    Open,
    Requested,
    Writing,
    Finished,
}
