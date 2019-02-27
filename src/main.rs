extern crate clap;
extern crate kvdb_rocksdb;
extern crate ethereum_types;
use rlp::{decode, encode, Decodable, Encodable};
#[macro_use]
extern crate log;
extern common_types as types;

use clap::App;
use kvdb_rocksdb::{Database, DatabaseConfig};
use ethereum_types::{H256};

fn fix_bft() {
unimplemented!();
}

fn fix_db() {
    unimplemented!();
}

/// Column for State
pub const COL_STATE: Option<u32> = Some(0);
/// Column for Block headers
pub const COL_HEADERS: Option<u32> = Some(1);
/// Column for Block bodies
pub const COL_BODIES: Option<u32> = Some(2);
/// Column for Extras
pub const COL_EXTRA: Option<u32> = Some(3);
/// Column for Traces
pub const COL_TRACE: Option<u32> = Some(4);
/// Column for the empty accounts bloom filter.
pub const COL_ACCOUNT_BLOOM: Option<u32> = Some(5);
/// Column for general information from the local node which can persist.
pub const COL_NODE_INFO: Option<u32> = Some(6);
/// Number of columns in DB
pub const NUM_COLUMNS: Option<u32> = Some(7);

pub struct CurrentHash;

/*impl Key<H256> for CurrentHash {
    type Target = H256;

    fn key(&self) -> H256 {
        H256::from("7cabfb7709b29c16d9e876e876c9988d03f9c3414e1d3ff77ec1de2d0ee59f66")
    }
}*/

fn main() {

    let matches = App::new("cita-recover")
        //.version(get_build_info_str(true))
        .author("yubo")
        .about("CITA Block Chain Node powered by Rust")
        .args_from_usage("-h, --height=[NUMBER] 'Sets the destinate height'")
        .args_from_usage("-d, --data=[PATH] 'Set data dir'")
        .get_matches();

    let data_path = matches.value_of("data").unwrap_or("./data");
    let chash = H256::from("7cabfb7709b29c16d9e876e876c9988d03f9c3414e1d3ff77ec1de2d0ee59f66");
    let cheight = H256::from("7cabfb7709b29c16d9e876e876c9988d03f9c3414e1d3ff77ec1de2d0ee59f68");

    let database_config = DatabaseConfig::with_columns(NUM_COLUMNS);
    let mut db = Database::open(&database_config,data_path).expect("DB file not found");
    match db.get(COL_EXTRA,&cheight) {
        Ok(Some(value)) => println!("retrieved value {:?}", value),
        Ok(None) => println!("value not found"),
        Err(e) => println!("operational problem encountered: {}", e),
    }


}
