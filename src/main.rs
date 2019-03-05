extern crate bincode;
extern crate cita_types;
extern crate clap;
extern crate common_types as types;
extern crate db as cita_db;
extern crate libproto;
extern crate log;
extern crate proof;
extern crate rlp;

use cita_db::{DBTransaction, Database, DatabaseConfig};
use cita_types::H256;
use clap::App;
/*use libproto::blockchain::{
    AccountGasLimit as ProtoAccountGasLimit, Proof as ProtoProof, ProofType,
};
use rlp::{decode, encode, Decodable, Encodable};
*/
use std::path::Path;
use types::db::{Readable, Writable};
use types::header::*;
use types::{db, extras, BlockNumber};

use bincode::{serialize, Infinite};
use proof::BftProof;
use std::fs::{read_dir, remove_file, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::mem::transmute;
//use types::db::Key;

const PTYPE: u8 = 5;
const HTYPE: u8 = 4;
const WAL: &str = "/wal";
const NOSQL: &str = "/nosql";
const STATEDB: &str = "/statedb";

fn delete_higher_log_file(data_path: &str, height: usize) {
    let wal_path = data_path.to_string() + WAL;
    if let Ok(entries) = read_dir(wal_path.clone()) {
        for entry in entries {
            if let Ok(entry) = entry {
                // Here, `entry` is a `DirEntry`.
                if let Ok(fname) = entry.file_name().into_string() {
                    let vec_str: Vec<&str> = fname.split(".log").collect();
                    if !vec_str.is_empty() {
                        let hi = vec_str[0].parse::<usize>().unwrap_or(0);
                        if hi > height {
                            let del_file = wal_path.clone() + "/" + &fname;
                            println!("del file name {:?}", del_file);
                            let _ = remove_file(del_file);
                        }
                    }
                }
            }
        }
    }
}

fn fix_wal_index(data_path: &str, height: usize) -> Result<usize, io::Error> {
    let idex_file = data_path.to_string() + WAL + "/index";
    let mut ifs = OpenOptions::new()
        .read(true)
        .create(true)
        .write(true)
        .open(idex_file)?;

    let mut string_buf: String = String::new();
    let _ = ifs.read_to_string(&mut string_buf)?;
    println!("Wal old index height {}", string_buf);

    let hstr = (height + 1).to_string().into_bytes();
    let len = hstr.len();
    ifs.seek(io::SeekFrom::Start(0)).unwrap();
    ifs.set_len(len as u64)?;
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
    let wal_path = data_path.to_string() + WAL + "/";
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
    let ptype_bytes: [u8; 1] = unsafe { transmute(PTYPE.to_le()) };
    let hlen_bytes: [u8; 4] = unsafe { transmute(hlen.to_le()) };
    let htype_bytes: [u8; 1] = unsafe { transmute(HTYPE.to_le()) };

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
    let exec_path = data_path.to_string() + STATEDB;
    if !Path::new(&exec_path).exists() {
        println!("executor db dir not exist");
        return false;
    }

    let database_config = DatabaseConfig::with_columns(db::NUM_COLUMNS);
    let exec_db = Database::open(&database_config, &*exec_path).expect("exec DB file not found");

    let hash: H256 = exec_db
        .read(db::COL_EXTRA, &extras::CurrentHash)
        .expect("CurrentHash value not found");;
    let hdr: Header = exec_db
        .read(db::COL_HEADERS, &hash)
        .expect("CurrentHeader value not found");

    if hdr.number() < dst_height {
        println!(
            " exec Dst height greater then current hight {}. Think about it",
            hdr.number()
        );
        return false;
    }

    let dst_hash: H256 = exec_db
        .read(db::COL_EXTRA, &dst_height)
        .expect("Dst Hash value not found");;

    let dst_header: Option<Header> = exec_db.read(db::COL_HEADERS, &dst_hash);

    if let Some(_dst_header) = dst_header {
        let mut batch = DBTransaction::new();
        batch.write(db::COL_EXTRA, &extras::CurrentHash, &dst_hash);
        exec_db.write(batch).unwrap();
        println!("write dst_hash is {:?}", dst_hash);
    } else {
        println!("Executor Dst header value not found");
        return false;
    }
    true
}

fn fix_chain_db(data_path: &str, dst_height: u64) -> bool {
    let chain_path = data_path.to_string() + NOSQL;

    if !Path::new(&chain_path).exists() {
        println!("chain db dir not exist");
        return false;
    }

    let database_config = DatabaseConfig::with_columns(db::NUM_COLUMNS);
    let chain_db = Database::open(&database_config, &*chain_path).expect("DB file not found");

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

    //Tmp,To be deleted
    /*{
        let hi = dst_height;
            let mut result = [0u8; 9];
            result[0] = extras::ExtrasIndex::BlockHeadHash as u8;
            result[1] = (hi >> 56) as u8;
            result[2] = (hi >> 48) as u8;
            result[3] = (hi >> 40) as u8;
            result[4] = (hi >> 32) as u8;
            result[5] = (hi >> 24) as u8;
            result[6] = (hi >> 16) as u8;
            result[7] = (hi >> 8) as u8;
            result[8] = (hi &0xff) as u8;

            let xx  = chain_db.get(db::COL_HEADERS, &result).unwrap().unwrap();
            println!("len get heads {:?} first {:?}",xx.len(),xx.first());

        result[0] = extras::ExtrasIndex::BlockBodyHash as u8;
        let yy  = chain_db.get(db::COL_BODIES, &result).unwrap().unwrap();
        println!("len get bodys {:?}",yy);

        let proof = dst_header.proof();
        let btf_proof = BftProof::from(proof.clone());

        println!("btf proof -- {:?}" ,btf_proof);

    }*/

    let nh: Option<Header> = chain_db.read(db::COL_HEADERS, &(dst_height + 1));
    if let Some(next_header) = nh {
        let proof = next_header.proof();
        let btf_proof = BftProof::from(proof.clone());
        if btf_proof.height == dst_height as usize {
            batch.write(db::COL_EXTRA, &extras::CurrentProof, &proof);

            let pmsg = serialize(&btf_proof, Infinite).unwrap();
            let hmsg = dst_hash.to_vec();
            let _ = fix_height_log(data_path, dst_height, pmsg, hmsg);
        }
    } else {
        println!(
            "current proof not inserted,as height's {} not found",
            dst_height
        );
    }

    chain_db.write(batch).unwrap();
    //println!("header is {:?}", dst_header);
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
        .unwrap_or("0")
        .to_string()
        .parse::<u64>()
        .unwrap_or(0);

    if dst_height == 0 {
        println!("height param must greater than 0");
        return;
    }

    delete_higher_log_file(data_path, dst_height as usize);
    let _ = fix_wal_index(data_path, dst_height as usize);

    if !fix_chain_db(data_path, dst_height) {
        return;
    }

    if !fix_executor_db(data_path, dst_height) {
        return;
    }
}
