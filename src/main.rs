extern crate cita_types;
extern crate clap;
extern crate db as cita_db;
extern crate rlp;

use rlp::{decode, encode, Decodable, Encodable};
#[macro_use]
extern crate log;
extern crate common_types as types;

use cita_db::{DBTransaction, Database, DatabaseConfig, KeyValueDB};
use cita_types::H256;
use clap::App;
use types::db::Readable;
use types::{db, extras, BlockNumber};

fn fix_bft() {
    unimplemented!();
}

fn fix_db() {
    unimplemented!();
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
        .unwrap_or("9")
        .to_string()
        .parse::<usize>()
        .unwrap_or(0);

    if dst_height == 0 {
        println!("height param must greater than 0");
        return;
    }

    let database_config = DatabaseConfig::with_columns(db::NUM_COLUMNS);
    let mut chain_db = Database::open(&database_config, data_path).expect("DB file not found");

    let hash: H256 = chain_db.read(db::COL_EXTRA, &extras::CurrentHash).expect("CurrentHash value not found");;
    let hi: BlockNumber = chain_db.read(db::COL_EXTRA, &hash).expect("CurrentHeight value not found");

    if hi as usize < dst_height {
        println!(" Dst height greater then current hight {}. Think about it",hi);
        return;
    }







}
