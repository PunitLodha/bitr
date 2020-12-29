use bitvec::{order::Msb0, prelude::BitVec};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::peer::Peer;
use crate::Result;
// TODO
// Create an mpsc channel and clone the transmitter and give it to all the tasks
// peers will send messages through this containing a oneshot transmitter
// manager will process the message and reply if necessary through the oneshot transmiter
// peer will  get the message using its oneshot receiver
#[derive(Debug)]
struct Manager {
    peer_list: Vec<Peer>,
    piece_picker: PiecePicker,
}

impl Manager {
    pub fn new(
        peer_list: Vec<Peer>,
        no_of_pieces: u32,
        piece_hashes: Vec<[u8; 20]>,
        piece_length: u32,
    ) -> Self {
        let piece_picker = PiecePicker::new(no_of_pieces, piece_hashes, piece_length);
        Self {
            peer_list,
            piece_picker,
        }
    }
    pub fn connect_to_peers(&self, info_hash: &Vec<u8>) -> Result<()> {
        for peer in self.peer_list.iter() {
            peer.connect(info_hash)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct PiecePicker {
    piece_map: Vec<PiecePos>,
    pieces: Vec<u32>,
    priority_boundaries: Vec<u32>,
    downloading: HashMap<u32, DownloadingPiece>,
    piece_hashes: Vec<[u8; 20]>,
    piece_length: u32,
}

impl PiecePicker {
    pub fn new(total_pieces: u32, piece_hashes: Vec<[u8; 20]>, piece_length: u32) -> Self {
        let piece_map = (0..total_pieces)
            .map(|index| PiecePos::new(0, PieceState::NotDownloading, index))
            .collect();
        let pieces = (0..total_pieces).collect();
        // priortity boundaries contains indexes of boundaries for different availabilties
        // maximum availabilty for a piece can be the amount of peers connected
        // 35 peers are mostly enough for a file, and we receive only 30 peers at a time from the tracker
        // so we assume a safe number of 50 max peers connected at a time
        let priority_boundaries = vec![total_pieces; 50];
        let downloading = HashMap::new();
        Self {
            piece_map,
            pieces,
            priority_boundaries,
            downloading,
            piece_hashes,
            piece_length,
        }
    }
    pub fn register_bitfield(&mut self, mut bitfield: BitVec<Msb0, u8>) {
        bitfield.resize(self.pieces.len(), false);

        // if bitfield has all pieces
        // todo find a better solution to update availability when bitfield has all pieces
        // try to somehow generalize the case

        for (piece, available_piece) in bitfield.iter().enumerate() {
            if *available_piece {
                self.increment_piece_availability(piece);
            }
        }
    }
    fn increment_piece_availability(&mut self, piece: usize) {
        let avail = self.piece_map[piece].peer_count;
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
    pub fn pick_piece(&mut self, peer_bitfield: BitVec<Msb0, u8>) -> Option<Block> {
        for piece in &self.pieces {
            // if peer has the piece
            if peer_bitfield[*piece as usize] {
                let downloading_piece = self
                    .downloading
                    .entry(*piece)
                    .or_insert(DownloadingPiece::new(*piece, self.piece_length));
                for block in downloading_piece.blocks.iter_mut() {
                    if let BlockState::Open = block.state {
                        block.state = BlockState::Requested;
                        return Some(Block::new(block.piece_index, block.begin));
                    }
                }
            }
        }
        None
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
struct DownloadingPiece {
    // kind of redundant, maybe remove it later
    index: u32,
    blocks: Vec<Block>,
}

impl DownloadingPiece {
    // todo check for last piece and make necessary changes to no of blocks
    fn new(index: u32, piece_length: u32) -> Self {
        const BLOCK_LENGTH: u32 = 16384;
        let no_of_blocks = piece_length / BLOCK_LENGTH;
        let blocks = (0..no_of_blocks)
            .map(|i| Block::new(index, i * 16384))
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
    piece_index: u32,
    /// zero-based byte offset within the piece
    begin: u32,
    length: u32,
    state: BlockState,
}

impl Block {
    fn new(piece_index: u32, begin: u32) -> Self {
        let length = 16384;
        let state = BlockState::Open;
        Self {
            piece_index,
            begin,
            length,
            state,
        }
    }
}

/*
#[cfg(test)]
mod tests {
    use bitvec::bitvec;

    use super::*;
    #[test]
    fn test_num() -> Result<()> {
        let bitfield = bitvec![Msb0, u8; 0,1,1,1,0,1,1,1,0,0,1,1,1,1,1];
        let vec = register_bitfield(bitfield);
        assert_eq!(vec, vec![1, 2]);
        Ok(())
    }
}
 */

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::bitvec;

    #[test]
    fn update_availablity_of_one_piece() -> Result<()> {
        let total_pieces = 3;
        let piece_length = 262144;
        let piece_hashes: Vec<[u8; 20]> = vec![];
        let mut piece_picker = PiecePicker::new(total_pieces, piece_hashes, piece_length);

        let bitfield = bitvec![Msb0,u8;0,1,0];
        piece_picker.register_bitfield(bitfield);
        assert_eq!(piece_picker.piece_map[1].index, 2);

        let bitfield = bitvec![Msb0,u8;0,1,0];
        piece_picker.register_bitfield(bitfield);
        assert_eq!(piece_picker.piece_map[1].index, 2);

        let bitfield = bitvec![Msb0,u8;1,0,0];
        piece_picker.register_bitfield(bitfield);
        assert_eq!(piece_picker.piece_map[0].index, 1);

        let bitfield = bitvec![Msb0,u8;1,1,1];
        piece_picker.register_bitfield(bitfield);
        assert_eq!(piece_picker.piece_map[0].index, 1);
        assert_eq!(piece_picker.piece_map[1].index, 2);
        assert_eq!(piece_picker.piece_map[2].index, 0);

        Ok(())
    }

    #[test]
    fn update_availablity_of_one_pieces() -> Result<()> {
        let b = bitvec![Msb0,u8;0,1,0,0,1];
        dbg!(b[1]);
        assert!(b[1]);
        let total_pieces = 5;
        let piece_length = 262144;
        let piece_hashes: Vec<[u8; 20]> = vec![];
        let mut piece_picker = PiecePicker::new(total_pieces, piece_hashes, piece_length);

        let bitfield = bitvec![Msb0,u8;0,1,0,0,1];
        piece_picker.register_bitfield(bitfield);

        let bitfield = bitvec![Msb0,u8;1,0,0,1,0];
        piece_picker.register_bitfield(bitfield);

        let bitfield = bitvec![Msb0,u8;0,1,0,1,0];
        piece_picker.register_bitfield(bitfield);

        let bitfield = bitvec![Msb0,u8;1,1,1,1,1];
        piece_picker.register_bitfield(bitfield);
        piece_picker.decrement_piece_availability(0);
        dbg!(piece_picker);

        Ok(())
    }
}
