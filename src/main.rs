extern crate cita_types;
extern crate clap;
extern crate db as cita_db;
extern crate libproto;
extern crate rlp;
#[macro_use]
extern crate log;
extern crate bincode;
extern crate common_types as types;
extern crate proof;

use cita_db::{DBTransaction, Database, DatabaseConfig, KeyValueDB};
use cita_types::H256;
use clap::App;
use libproto::blockchain::{
    AccountGasLimit as ProtoAccountGasLimit, Proof as ProtoProof, ProofType,
};
use rlp::{decode, encode, Decodable, Encodable};
use std::path::Path;
use types::db::{Readable, Writable};
use types::header::*;
use types::{db, extras, BlockNumber};

use bincode::{deserialize, serialize, Infinite};
use proof::BftProof;
use std::fs::{read_dir, remove_file, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::mem::transmute;

const ptype: u8 = 5;
const htype: u8 = 4;

fn delete_higher_log_file(wal_path: &str, height: usize) {
    if let Ok(entries) = read_dir(wal_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                // Here, `entry` is a `DirEntry`.
                if let Ok(fname) = entry.file_name().into_string() {
                    let vec_str: Vec<&str> = fname.split(".log").collect();
                    if !vec_str.is_empty() {
                        let hi = vec_str[0].parse::<usize>().unwrap_or(0);
                        if hi > height {
                            let _ = remove_file(fname.clone());
                        }
                    }
                }
            }
        }
    }
}

fn fix_wal_index(wal_path: &str, height: usize) -> Result<usize, io::Error> {
    let idex_file = wal_path.to_string() + "/index";
    let mut ifs = OpenOptions::new()
        .read(true)
        .create(true)
        .write(true)
        .open(idex_file)?;
    ifs.seek(io::SeekFrom::Start(0)).unwrap();

    let mut string_buf: String = String::new();
    let _ = ifs.read_to_string(&mut string_buf)?;
    println!("Wal old index height {}", string_buf);

    let hstr = (height + 1).to_string().into_bytes();
    let len = hstr.len();
    ifs.set_len(len as u64);
    ifs.write_all(hstr.as_slice())?;
    ifs.flush()?;
    println!("Wal write index new height {}", height + 1);
    Ok(len)
}

fn fix_height_log(
    data_path: &str,
    dst_height: u64,
    proof: Vec<u8>,
    hash: Vec<u8>,
) -> Result<usize, io::Error> {
    let wal_path = data_path.to_string() + "/wal/";
    let hstr = (dst_height + 1).to_string();
    let fname = wal_path + &hstr + ".log";

    let mut fs = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(fname)?;
    fs.seek(io::SeekFrom::Start(0))?;

    let plen = proof.len() as u32;
    let hlen = hash.len() as u32;

    let plen_bytes: [u8; 4] = unsafe { transmute(plen.to_le()) };
    let ptype_bytes: [u8; 1] = unsafe { transmute(ptype.to_le()) };
    let hlen_bytes: [u8; 4] = unsafe { transmute(hlen.to_le()) };
    let htype_bytes: [u8; 1] = unsafe { transmute(htype.to_le()) };

    fs.write_all(&hlen_bytes[..])?;
    fs.write_all(&htype_bytes[..])?;
    fs.write_all(&hash)?;

    fs.write_all(&plen_bytes[..])?;
    fs.write_all(&ptype_bytes[..])?;
    fs.write_all(&proof)?;
    fs.flush()?;
    Ok((plen + hlen + (4 + 1) * 2) as usize)
}

fn fix_executor_db(data_path: &str, dst_height: u64) -> bool {
    let exec_path = data_path.to_string() + "/statedb";
    if !Path::new(&exec_path).exists() {
        println!("executor db dir not exist");
        return false;
    }

    let database_config = DatabaseConfig::with_columns(db::NUM_COLUMNS);
    let mut exec_db =
        Database::open(&database_config, &*exec_path).expect("exec DB file not found");

    let hash: H256 = exec_db
        .read(db::COL_EXTRA, &extras::CurrentHash)
        .expect("CurrentHash value not found");;
    let hi: BlockNumber = exec_db
        .read(db::COL_EXTRA, &hash)
        .expect("CurrentHeight value not found");

    if hi < dst_height {
        println!(
            " exec Dst height greater then current hight {}. Think about it",
            hi
        );
        return false;
    }

    let dst_header: Header = exec_db
        .read(db::COL_HEADERS, &dst_height)
        .expect("Executor Dst header value not found");
    let dst_hash = dst_header.hash().unwrap();

    let mut batch = DBTransaction::new();

    batch.write(db::COL_EXTRA, &extras::CurrentHash, &dst_hash);
    exec_db.write(batch);
    println!("dst_hash is {:?}", dst_hash);
    true
}

fn fix_chain_db(data_path: &str, dst_height: u64) -> bool {
    let chain_path = data_path.to_string() + "/nosql";

    if !Path::new(&chain_path).exists() {
        println!("chain db dir not exist");
        return false;
    }

    let database_config = DatabaseConfig::with_columns(db::NUM_COLUMNS);
    let mut chain_db = Database::open(&database_config, &*chain_path).expect("DB file not found");

    let hash: H256 = chain_db
        .read(db::COL_EXTRA, &extras::CurrentHash)
        .expect("CurrentHash value not found");;
    let hi: BlockNumber = chain_db
        .read(db::COL_EXTRA, &hash)
        .expect("CurrentHeight value not found");

    if hi < dst_height {
        println!(
            " Dst height greater then current hight {}. Think about it",
            hi
        );
        return false;
    }

    let dst_header: Header = chain_db
        .read(db::COL_HEADERS, &dst_height)
        .expect("Dst header value not found");
    let dst_hash = dst_header.hash().unwrap();

    let mut batch = DBTransaction::new();

    batch.write(db::COL_EXTRA, &extras::CurrentHash, &dst_hash);

    let nh: Option<Header> = chain_db.read(db::COL_HEADERS, &(dst_height + 1));
    if let Some(next_header) = nh {
        let proof = next_header.proof();
        let btf_proof = BftProof::from(proof.clone());
        if btf_proof.height == dst_height as usize {
            batch.write(db::COL_EXTRA, &extras::CurrentProof, &proof);

            let pmsg = serialize(&btf_proof, Infinite).unwrap();
            let hmsg = dst_hash.to_vec();
            fix_height_log(data_path, dst_height + 1, pmsg, hmsg);
        }
    } else {
        println!(
            "current proof not inserted,as height's {} not found",
            dst_height + 1
        );
    }

    chain_db.write(batch);
    println!("header is {:?}", dst_header);
    println!("dst_hash is {:?}", dst_hash);
    true
}

fn main() {
    let matches = App::new("cita-recover")
        //.version(get_build_info_str(true))
        .author("yubo")
        .about("CITA Block Chain Node powered by Rust")
        .args_from_usage("-h, --height=[NUMBER] 'Sets the destinate height'")
        .args_from_usage("-d, --data=[PATH] 'Set data dir'")
        .get_matches();

    let data_path = matches.value_of("data").unwrap_or("./data");
    let dst_height = matches
        .value_of("height")
        .unwrap_or("39")
        .to_string()
        .parse::<u64>()
        .unwrap_or(0);

    if dst_height == 0 {
        println!("height param must greater than 0");
        return;
    }

    delete_higher_log_file(data_path, dst_height as usize);
    fix_wal_index(data_path, dst_height as usize);

    if !fix_chain_db(data_path, dst_height) {
        return;
    }

    if !fix_executor_db(data_path, dst_height) {
        return;
    }
}
