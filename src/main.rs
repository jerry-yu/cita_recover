extern crate cita_types;
extern crate clap;
extern crate db as cita_db;
extern crate libproto;
extern crate rlp;

use rlp::{decode, encode, Decodable, Encodable};
#[macro_use]
extern crate log;
extern crate common_types as types;

use cita_db::{DBTransaction, Database, DatabaseConfig, KeyValueDB};
use cita_types::H256;
use clap::App;
use libproto::blockchain::{
    AccountGasLimit as ProtoAccountGasLimit, Proof as ProtoProof, ProofType,
};
use std::path::Path;
use types::db::{Readable, Writable};
use types::header::*;
use types::{db, extras, BlockNumber};

fn fix_bft() {
    unimplemented!();
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

    let hash: H256 = chain_db
        .read(db::COL_EXTRA, &extras::CurrentHash)
        .expect("CurrentHash value not found");;
    let hi: BlockNumber = chain_db
        .read(db::COL_EXTRA, &hash)
        .expect("CurrentHeight value not found");

    if hi < dst_height {
        println!(
            " exec Dst height greater then current hight {}. Think about it",
            hi
        );
        return false;
    }

    let dst_header: Header = chain_db
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
        batch.write(db::COL_EXTRA, &extras::CurrentProof, &(next_header.proof()));
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

    if !fix_chain_db(data_path, dst_height) {
        return;
    }
}
