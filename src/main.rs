use bincode;
use cita_types;
use clap;
use common_types as types;

use types::db_indexes::{
    BlockNumber2Hash, BlockNumber2Header, CurrentHash, CurrentHeight, CurrentProof, DBIndex,
    Hash2BlockNumber, Hash2Header,
};

use cita_types::H256;
use clap::App;

use cita_database::{Config, DataCategory, Database, RocksDB, NUM_COLUMNS};

/*use libproto::blockchain::{
    AccountGasLimit as ProtoAccountGasLimit, Proof as ProtoProof, ProofType,
};
use rlp::{decode, encode, Decodable, Encodable};
*/
use std::path::Path;
// use types::db::{Readable, Writable};
use types::header::*;
// use types::{db, extras, BlockNumber};

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

    let database_config = Config::with_category_num(NUM_COLUMNS);
    let exec_db = RocksDB::open(&exec_path, &database_config).expect("exec DB file not found");

    let hash = exec_db
        .get(Some(DataCategory::Extra), &CurrentHash.get_index().to_vec())
        .unwrap_or(None)
        .map(|h| rlp::decode::<H256>(&h))
        .expect("current hash not get");

    let hash_key = Hash2Header(hash).get_index();

    let hdr = exec_db
        .get(Some(DataCategory::Headers), &hash_key)
        .unwrap_or(None)
        .map(|h| rlp::decode::<Header>(&h))
        .expect("hashe's header not found");

    if hdr.number() < dst_height {
        println!(
            "WARN exec Dst height greater then current hight {}. Think about it",
            hdr.number()
        );
        // this check should be done
        //return false;
    }

    let pkey = BlockNumber2Hash(dst_height).get_index().to_vec();
    let dst_hash = exec_db
        .get(Some(DataCategory::Extra), &pkey)
        .unwrap_or(None)
        .map(|h| rlp::decode::<H256>(&h))
        .expect("dst hash not get");

    let hash_header_key = Hash2Header(dst_hash).get_index().to_vec();

    let _dst_header = exec_db
        .get(Some(DataCategory::Headers), &hash_header_key)
        .unwrap_or(None)
        .map(|h| rlp::decode::<H256>(&h))
        .expect("dst header not get");

    exec_db
        .insert(
            Some(DataCategory::Extra),
            CurrentHash.get_index().to_vec(),
            rlp::encode(&dst_hash).into_vec(),
        )
        .expect("write current hash error");
    println!("write dst_hash is {:?}", dst_hash);
    true
}

fn fix_chain_db(data_path: &str, dst_height: u64) -> bool {
    let chain_path = data_path.to_string() + NOSQL;

    if !Path::new(&chain_path).exists() {
        println!("chain db dir not exist");
        return false;
    }

    let database_config = Config::with_category_num(NUM_COLUMNS);
    let chain_db = RocksDB::open(&chain_path, &database_config).expect("DB file not found");

    /*    let hash: H256 = chain_db
            .read(db::COL_EXTRA, &extras::CurrentHash)
            .expect("CurrentHash value not found");
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
    */

    let hkey = BlockNumber2Header(dst_height).get_index().to_vec();

    let dst_header: Header = chain_db
        .get(Some(DataCategory::Headers), &hkey)
        .unwrap_or(None)
        .map(|hdr| rlp::decode(&hdr))
        .expect("Dst header value not found");

    let dst_hash = dst_header.hash().unwrap();

    chain_db
        .insert(
            Some(DataCategory::Extra),
            CurrentHash.get_index().to_vec(),
            rlp::encode(&dst_hash).into_vec(),
        )
        .expect("chain write current hash error");

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

    let hash_key = BlockNumber2Header(dst_height + 1).get_index();
    let next_header = chain_db
        .get(Some(DataCategory::Headers), &hash_key)
        .unwrap_or(None)
        .map(|h| rlp::decode::<Header>(&h))
        .expect("current proof not inserted,as height's not found");

    let proof = next_header.proof();
    let btf_proof = BftProof::from(proof.clone());
    if btf_proof.height == dst_height as usize {
        chain_db
            .insert(
                Some(DataCategory::Extra),
                CurrentProof.get_index().to_vec(),
                rlp::encode(proof).into_vec(),
            )
            .expect("chain write current proof error");

        let pmsg = serialize(&btf_proof, Infinite).unwrap();
        let hmsg = dst_hash.to_vec();
        let _ = fix_height_log(data_path, dst_height, pmsg, hmsg);
    }

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
        println!("Not Success in chain db");
        return;
    }

    if !fix_executor_db(data_path, dst_height) {
        println!("Not Success in executor db");
        return;
    }
    println!("Done");
}
