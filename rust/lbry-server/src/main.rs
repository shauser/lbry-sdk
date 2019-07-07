use dirs;
use std::path::PathBuf;
use std::fs;
use std::io::Cursor;
use std::u32;
//use glob;

use bitcoin_hashes::sha256d;
use bitcoin::consensus::encode::{deserialize, Decodable};
use bitcoin::blockdata::transaction::Transaction;

/// A block header, which contains all the block's information except
/// the actual transactions
#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub struct BlockHeader {
    /// The protocol version. Should always be 1.
    pub version: u32,
    /// Reference to the previous block in the chain
    pub prev_blockhash: sha256d::Hash,
    /// The root hash of the merkle tree of transactions in the block
    pub merkle_root: sha256d::Hash,
    /// The root hash of the merkle tree of transactions in the block
    pub claim_trie_root: sha256d::Hash,
    /// The timestamp of the block, as claimed by the miner
    pub time: u32,
    /// The target value below which the blockhash must lie, encoded as a
    /// a float (with well-defined rounding, of course)
    pub bits: u32,
    /// The nonce, selected to obtain a low enough blockhash
    pub nonce: u32,
}

/// A Bitcoin block, which is a collection of transactions with an attached
/// proof of work.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Block {
    /// The block header
    pub header: BlockHeader,
    /// List of transactions contained in the block
    pub txdata: Vec<Transaction>
}

macro_rules! impl_consensus_encoding {
    ($thing:ident, $($field:ident),+) => (
        impl<S: bitcoin::consensus::encode::Encoder> bitcoin::consensus::encode::Encodable<S> for $thing {
            #[inline]
            fn consensus_encode(&self, s: &mut S) -> Result<(), bitcoin::consensus::encode::Error> {
                $( self.$field.consensus_encode(s)?; )+
                Ok(())
            }
        }

        impl<D: bitcoin::consensus::encode::Decoder> bitcoin::consensus::encode::Decodable<D> for $thing {
            #[inline]
            fn consensus_decode(d: &mut D) -> Result<$thing, bitcoin::consensus::encode::Error> {
                use bitcoin::consensus::encode::Decodable;
                Ok($thing {
                    $( $field: Decodable::consensus_decode(d)?, )+
                })
            }
        }
    );
}

impl_consensus_encoding!(BlockHeader, version, prev_blockhash, merkle_root, claim_trie_root, time, bits, nonce);
impl_consensus_encoding!(Block, header, txdata);

fn main() {
    /*let path = PathBuf::from("/home/lex/.lbrycrd/blocks/blk*.dat");
    println!("lbrycrd blocks path: {:?}", path);
    for blk in glob::glob(path.to_str().unwrap()).unwrap() {
        println!("{:?}", blk.unwrap().display());
    }*/
    let path = dirs::home_dir().unwrap().join(".lbrycrd/blocks/blk00000.dat");
    let blob = fs::read(&path).unwrap();
    let mut cursor = Cursor::new(&blob);
    let mut blocks = vec![];
    let magic = 4054508794;
    let max_pos = blob.len() as u64;
    while cursor.position() < max_pos {
        let offset = cursor.position();
        match u32::consensus_decode(&mut cursor) {
            Ok(value) => {
                if magic != value {
                    cursor.set_position(offset + 1);
                    continue;
                }
            }
            Err(_) => break, // EOF
        };
        let block_size = u32::consensus_decode(&mut cursor).unwrap();
        let start = cursor.position();
        let end = start + block_size as u64;

        // If Core's WriteBlockToDisk ftell fails, only the magic bytes and size will be written
        // and the block body won't be written to the blk*.dat file.
        // Since the first 4 bytes should contain the block's version, we can skip such blocks
        // by peeking the cursor (and skipping previous `magic` and `block_size`).
        match u32::consensus_decode(&mut cursor) {
            Ok(value) => {
                if magic == value {
                    cursor.set_position(start);
                    continue;
                }
            }
            Err(_) => break, // EOF
        }
        let block: Block = deserialize(&blob[start as usize..end as usize]).unwrap();
        println!("{} txs: {}", blocks.len(), block.txdata.len());
        blocks.push(block);
        cursor.set_position(end as u64);
    }
}
