use bitvec::{order::Msb0, prelude::BitVec};
use ring::digest;
use std::path::PathBuf;
use std::{collections::HashMap, u32};
use std::{convert::TryInto, usize};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};
use tokio::task::JoinHandle;

use crate::{disk::DiskManager, peer::Peer, torrent::Torrent, tracker, utils};
use crate::{tracker::TrackerResponse, Result};
// TODO
// Create an mpsc channel and clone the transmitter and give it to all the tasks
// peers will send messages through this containing a oneshot transmitter
// manager will process the message and reply if necessary through the oneshot transmiter
// peer will  get the message using its oneshot receiver
#[derive(Debug)]
pub struct Manager {
    path: PathBuf,
    client_peer_id: Vec<u8>,
    //peer_list: Vec<Peer>,
    torrent: Torrent,
    //pub piece_picker: PiecePicker,
}

impl Manager {
    pub fn new(file_path: String) -> Result<Manager> {
        // path of the torrent file
        let path = PathBuf::from(file_path);
        // generate the peer id
        let client_peer_id = utils::generate_peer_id()?;

        let torrent = Torrent::new(&path)?;
        Ok(Manager {
            path,
            client_peer_id,
            torrent,
        })
    }
    pub fn send_tracker_request(&self) -> Result<TrackerResponse> {
        let url = self.torrent.generate_tracker_url(&self.client_peer_id)?;
        println!("{}", url.as_str());

        let res = tracker::send_tracker_request(url)?;
        Ok(res)
    }
    pub fn spawn_piece_picker(
        &self,
        send_to_disk_manager: UnboundedSender<DownloadedPiece>,
    ) -> PiecePicker {
        let pieces = self.torrent.info.pieces.to_vec();
        let piece_hashes: Vec<[u8; 20]> = pieces
            .chunks_exact(20)
            // try to remove this unwrap
            .map(|chunk| chunk.try_into().unwrap())
            .collect();
        let total_pieces = piece_hashes.len();
        println!("{}", total_pieces);

        let piece_length = self.torrent.info.piece_length;
        let file_length = self.torrent.info.length.unwrap();

        let piece_picker = PiecePicker::new(
            total_pieces as u32,
            piece_hashes,
            piece_length as u32,
            file_length as u32,
            send_to_disk_manager,
        );
        piece_picker
    }
    pub fn spawn_disk_manager(
        &self,
        receive_pieces: UnboundedReceiver<DownloadedPiece>,
    ) -> Result<DiskManager> {
        let disk_manager = DiskManager::new(
            receive_pieces,
            &self.torrent.info.name,
            self.torrent.info.piece_length,
            (self.torrent.info.pieces.to_vec().len() / 20) as u32,
        )?;
        Ok(disk_manager)
    }
    pub fn connect_to_peers(
        &self,
        res: TrackerResponse,
        send_to_manager: UnboundedSender<Command>,
    ) -> Vec<JoinHandle<()>> {
        let handles: Vec<JoinHandle<()>> = res
            .peers
            .into_iter()
            .map(|tracker_peer| {
                let mut peer = Peer::new(
                    tracker_peer.ip,
                    tracker_peer.port,
                    tracker_peer.peer_id.to_vec(),
                    send_to_manager.clone(),
                );
                let info = self.torrent.info_hash.clone();
                let client_peer_id = self.client_peer_id.clone();
                tokio::spawn(async move {
                    if let Err(e) = peer.connect(&info, &client_peer_id).await {
                        eprintln!("Some error occured:- {:?}", e);
                        eprintln!("Closing the connection");
                    };
                })
            })
            .collect();
        handles
    }
}

#[derive(Debug)]
pub struct PiecePicker {
    file_length: u32,
    total_pieces: u32,
    piece_map: Vec<PiecePos>,
    pieces: Vec<u32>,
    priority_boundaries: Vec<u32>,
    downloading: HashMap<u32, DownloadingPiece>,
    piece_hashes: Vec<[u8; 20]>,
    piece_length: u32,
    pub peer_bitfields: HashMap<Vec<u8>, BitVec<Msb0, u8>>,
    send_to_disk_manager: UnboundedSender<DownloadedPiece>,
    downloaded_pieces: HashMap<u32, DownloadedPiece>,
}

impl PiecePicker {
    pub fn new(
        total_pieces: u32,
        piece_hashes: Vec<[u8; 20]>,
        piece_length: u32,
        file_length: u32,
        send_to_disk_manager: UnboundedSender<DownloadedPiece>,
    ) -> Self {
        let piece_map = (0..total_pieces)
            .map(|index| PiecePos::new(0, PieceState::NotDownloading, index))
            .collect();
        let pieces = (0..total_pieces).collect();
        // priortity boundaries contains indexes of boundaries for different availabilties
        // maximum availabilty for a piece can be the amount of peers connected
        // 35 peers are mostly enough for a file, and we receive only 30 peers at a time from the tracker
        // so we assume a safe number of 50 max peers connected at a time
        let priority_boundaries = vec![total_pieces; 50];
        Self {
            file_length,
            total_pieces,
            piece_map,
            pieces,
            priority_boundaries,
            downloading: HashMap::new(),
            piece_hashes,
            piece_length,
            peer_bitfields: HashMap::new(),
            send_to_disk_manager,
            downloaded_pieces: HashMap::new(),
        }
    }
    pub fn register_bitfield(&mut self, peer_id: Vec<u8>, mut bitfield: BitVec<Msb0, u8>) {
        bitfield.resize(self.pieces.len(), false);

        // if bitfield has all pieces
        // todo find a better solution to update availability when bitfield has all pieces
        // try to somehow generalize the case
        for (piece, available_piece) in bitfield.iter().enumerate() {
            if *available_piece {
                if piece == 0 {
                    println!("{}, {:?}", piece, peer_id);
                }
                self.increment_piece_availability(piece);
            }
        }
        self.peer_bitfields.insert(peer_id, bitfield);
    }
    pub fn increment_piece_availability(&mut self, piece: usize) {
        let avail = self.piece_map[piece].peer_count;
        if piece == 0 {
            println!("{}, {}", piece, avail);
        }
        self.priority_boundaries[avail as usize] -= 1;
        self.piece_map[piece].peer_count += 1;
        let piece_index = self.piece_map[piece].index;
        let other_index = self.priority_boundaries[avail as usize];
        let other_piece = self.pieces[other_index as usize];
        self.pieces.swap(other_index as usize, piece_index as usize);
        // swap indexes
        let t = self.piece_map[piece].index;
        self.piece_map[piece].index = self.piece_map[other_piece as usize].index;
        self.piece_map[other_piece as usize].index = t;
    }
    fn decrement_piece_availability(&mut self, piece: usize) {
        self.piece_map[piece].peer_count -= 1;
        let avail = self.piece_map[piece].peer_count;
        let piece_index = self.piece_map[piece].index;
        let other_index = self.priority_boundaries[avail as usize];
        let other_piece = self.pieces[other_index as usize];
        self.pieces.swap(other_index as usize, piece_index as usize);
        // swap indexes
        let t = self.piece_map[piece].index;
        self.piece_map[piece].index = self.piece_map[other_piece as usize].index;
        self.piece_map[other_piece as usize].index = t;

        self.priority_boundaries[avail as usize] += 1;
    }
    pub fn pick_intial_pieces(&mut self, peer_id: &Vec<u8>) -> Option<Vec<Option<Block>>> {
        let pieces: Vec<Option<Block>> = (0..5).map(|_| self.pick_piece(&peer_id)).collect();
        let no_piece = pieces.iter().all(|block| block.is_none());
        if no_piece {
            None
        } else {
            Some(pieces)
        }
    }
    pub fn pick_piece(&mut self, peer_id: &Vec<u8>) -> Option<Block> {
        let peer_bitfield = self.peer_bitfields.get(peer_id)?;
        let mut selected_index = self.pieces.len();
        let mut selected_block: Option<Block> = None;

        for (index, piece) in self.pieces.iter().enumerate() {
            // if peer has the piece
            if peer_bitfield[*piece as usize] {
                // check if final piece
                let piece_length = if index as u32 == self.total_pieces - 1 {
                    self.file_length - (index as u32 * self.piece_length)
                } else {
                    self.piece_length
                };
                let downloading_piece = self
                    .downloading
                    .entry(*piece)
                    .or_insert(DownloadingPiece::new(*piece, piece_length));

                for block in downloading_piece.blocks.iter_mut() {
                    if let BlockState::Open = block.state {
                        selected_index = index;
                        block.state = BlockState::Requested;
                        selected_block = Some(Block::new(
                            block.piece_index,
                            block.begin,
                            Some(block.length),
                        ));
                        break;
                    }
                }

                if selected_block.is_some() {
                    break;
                }
            }
        }

        if selected_index != self.pieces.len() {
            self.priortize_downloading_piece(selected_index);
        }

        selected_block
    }
    /// move the downloading piece at start
    /// index is the index of the piece in the pieces vector
    fn priortize_downloading_piece(&mut self, index: usize) {
        let removed_piece = self.pieces.remove(index);
        self.pieces.insert(0, removed_piece);

        // udpate piece map for the removed piece
        self.piece_map[removed_piece as usize].index = 0;

        // piece was removed and inserted at begining
        // so piece map and priority boundaries need to be updated
        for boundary in self.priority_boundaries.iter_mut() {
            if *boundary as usize <= index {
                *boundary += 1;
            }
        }
        // only elements from 1..index need to be updated as they are shifted to right by 1
        for i in 1..=index {
            let curr_piece = self.pieces[i];
            self.piece_map[curr_piece as usize].index += 1;
        }
    }
    pub async fn listen_to_commands(&mut self, mut receive_from_peers: UnboundedReceiver<Command>) {
        while let Some(cmd) = receive_from_peers.recv().await {
            match cmd {
                Command::BitfieldRecieved { peer_id, bitfield } => {
                    self.register_bitfield(peer_id, bitfield);
                    //println!("Recieved bitfield from peer");
                }
                Command::PickInitialPieces {
                    peer_id,
                    transmitter,
                } => {
                    let blocks = self.pick_intial_pieces(&peer_id);
                    match blocks {
                        Some(blks) => {
                            if let Err(_) = transmitter.send(Command::SelectedInitialPieces(blks)) {
                                eprintln!("Receiver Dropped");
                            };
                        }
                        None => {
                            if let Err(_) = transmitter.send(Command::NoPiece) {
                                eprintln!("Receiver Dropped");
                            };
                        }
                    }
                }
                Command::PickPiece {
                    peer_id,
                    transmitter,
                } => {
                    let block = self.pick_piece(&peer_id);
                    match block {
                        Some(blk) => {
                            if let Err(_) = transmitter.send(Command::SelectedPiece(blk)) {
                                eprintln!("Receiver Dropped");
                            };
                        }
                        None => {
                            if let Err(_) = transmitter.send(Command::NoPiece) {
                                eprintln!("Receiver Dropped");
                            };
                        }
                    }
                }
                Command::HavePiece {
                    peer_id,
                    piece_index,
                } => {
                    let mut already_has_piece = false;
                    if let Some(bitfield) = self.peer_bitfields.get_mut(&peer_id) {
                        if !(*bitfield.get_mut(piece_index).unwrap()) {
                            *bitfield.get_mut(piece_index).unwrap() = true;
                        } else {
                            already_has_piece = true
                        };
                    }
                    if !already_has_piece {
                        self.increment_piece_availability(piece_index);
                    }
                }
                Command::DownloadedBlock(block) => {
                    let index = block.piece_index;
                    // check if final piece
                    let piece_length = if index as u32 == self.total_pieces - 1 {
                        self.file_length - (index as u32 * self.piece_length)
                    } else {
                        self.piece_length
                    };
                    let downloaded_piece = self
                        .downloaded_pieces
                        .entry(index)
                        .or_insert(DownloadedPiece::new(index, piece_length));
                    downloaded_piece.add_downloaded_block(block);
                    if downloaded_piece.all_blocks_downloaded {
                        let piece_data =
                            downloaded_piece.blocks.iter().fold(vec![], |mut acc, blk| {
                                acc.extend_from_slice(&blk.data);
                                acc
                            });
                        let sha1 = digest::digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, &piece_data);
                        let sha1 = sha1.as_ref();
                        //check if the hash matches
                        if sha1 == self.piece_hashes.get(index as usize).unwrap() {
                            let piece = self.downloaded_pieces.remove(&index).unwrap();
                            if let Err(_) = self.send_to_disk_manager.send(piece) {
                                eprintln!("Receiver Dropped");
                            };
                        }
                        // todo implement part where hashes dont match
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
struct PiecePos {
    peer_count: u32,
    state: PieceState,
    index: u32,
}

impl PiecePos {
    fn new(peer_count: u32, state: PieceState, index: u32) -> Self {
        Self {
            peer_count,
            state,
            index,
        }
    }
}

#[derive(Debug)]
enum PieceState {
    Downloading,
    NotDownloading,
}

#[derive(Debug)]
pub struct DownloadingPiece {
    // kind of redundant, maybe remove it later
    index: u32,
    blocks: Vec<Block>,
}

impl DownloadingPiece {
    // todo check for last piece and make necessary changes to no of blocks
    fn new(index: u32, piece_length: u32) -> Self {
        const BLOCK_LENGTH: u32 = 16384;
        let mut no_of_blocks = piece_length / BLOCK_LENGTH;
        let final_block_len = piece_length % BLOCK_LENGTH;
        if final_block_len != 0 {
            no_of_blocks += 1
        }
        let blocks = (0..no_of_blocks)
            .map(|i| {
                if (final_block_len != 0) && (index == no_of_blocks - 1) {
                    Block::new(index, i * 16384, Some(final_block_len))
                } else {
                    Block::new(index, i * 16384, None)
                }
            })
            .collect();
        Self { index, blocks }
    }
}

#[derive(Debug)]
enum BlockState {
    Open,
    Requested,
    Writing,
    Finished,
}

#[derive(Debug)]
pub struct Block {
    /// the piece which the block belongs to
    pub piece_index: u32,
    /// zero-based byte offset within the piece
    pub begin: u32,
    pub length: u32,
    state: BlockState,
}

impl Block {
    fn new(piece_index: u32, begin: u32, block_length: Option<u32>) -> Self {
        let length = block_length.unwrap_or(16384);
        let state = BlockState::Open;
        Self {
            piece_index,
            begin,
            length,
            state,
        }
    }
}

#[derive(Debug)]
pub struct DownloadedBlock {
    /// the piece which the block belongs to
    pub piece_index: u32,
    /// zero-based byte offset within the piece
    pub begin: u32,
    pub data: Vec<u8>,
}

impl DownloadedBlock {
    pub fn new(piece_index: u32, begin: u32, data: Vec<u8>) -> Self {
        Self {
            piece_index,
            begin,
            data,
        }
    }
}

#[derive(Debug)]
pub struct DownloadedPiece {
    pub index: u32,
    pub blocks: Vec<DownloadedBlock>,
    all_blocks_downloaded: bool,
}

impl DownloadedPiece {
    pub fn new(index: u32, piece_length: u32) -> Self {
        const BLOCK_LENGTH: u32 = 16384;
        let mut no_of_blocks = piece_length / BLOCK_LENGTH;
        let final_block_len = piece_length % BLOCK_LENGTH;
        if final_block_len != 0 {
            no_of_blocks += 1
        }
        let blocks = (0..no_of_blocks)
            .map(|i| DownloadedBlock::new(index, i * 16384, vec![]))
            .collect();
        Self {
            index,
            blocks,
            all_blocks_downloaded: false,
        }
    }
    pub fn add_downloaded_block(&mut self, block: DownloadedBlock) {
        let block_index = block.begin / 16384;
        if let Some(blk) = self.blocks.get_mut(block_index as usize) {
            blk.data = block.data;
        }
        let all_blocks_downloaded = self.blocks.iter().all(|block| !block.data.is_empty());
        self.all_blocks_downloaded = all_blocks_downloaded;
    }
}

/// Commands that will be sent over the Message Channel
#[derive(Debug)]
pub enum Command {
    BitfieldRecieved {
        peer_id: Vec<u8>,
        bitfield: BitVec<Msb0, u8>,
    },
    PickInitialPieces {
        peer_id: Vec<u8>,
        transmitter: oneshot::Sender<Command>,
    },
    PickPiece {
        peer_id: Vec<u8>,
        transmitter: oneshot::Sender<Command>,
    },
    SelectedInitialPieces(Vec<Option<Block>>),
    SelectedPiece(Block),
    NoPiece,
    DownloadedBlock(DownloadedBlock),
    HavePiece {
        peer_id: Vec<u8>,
        piece_index: usize,
    },
}
