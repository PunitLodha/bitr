use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::os::unix::prelude::FileExt;
use tokio::task::{self, spawn_blocking, JoinHandle};

use tokio::sync::mpsc::UnboundedReceiver;

use crate::{manager::DownloadedPiece, Result};

pub struct DiskManager {
    receive_pieces: UnboundedReceiver<DownloadedPiece>,
    file: File,
    piece_length: u64,
    total_pieces: u32,
    completed_pieces: u32,
}

impl DiskManager {
    pub fn new(
        receive_pieces: UnboundedReceiver<DownloadedPiece>,
        file_name: &str,
        piece_length: u64,
        total_pieces: u32,
    ) -> Result<Self> {
        let file = File::create(file_name)?;
        Ok(Self {
            file,
            receive_pieces,
            piece_length,
            total_pieces,
            completed_pieces: 0,
        })
    }

    pub fn listen_for_pieces(mut self) -> JoinHandle<()> {
        let handle = task::spawn(async move {
            while let Some(piece) = self.receive_pieces.recv().await {
                let piece_data = piece.blocks.iter().fold(vec![], |mut acc, blk| {
                    acc.extend_from_slice(&blk.data);
                    acc
                });
                match self
                    .file
                    .write_all_at(&piece_data, (piece.index as u64) * self.piece_length)
                {
                    Err(e) => println!("Some err piece #{}", piece.index),
                    Ok(_) => {
                        self.completed_pieces += 1;
                        println!(
                            "Downloaded:- {:.9}% {} out of {}",
                            self.completed_pieces as f32 / self.total_pieces as f32,
                            piece.index,
                            self.total_pieces
                        );
                        //println!("Wrote piece #{}", piece.index)
                    }
                };
            }
        });
        handle
    }
}
