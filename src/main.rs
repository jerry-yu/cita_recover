extern crate clap;
extern crate rocksdb;
extern crate ethereum-types;
#[macro_use]
extern crate log;

use clap::App;
use rocksdb::{DB};
use ethereum::{H256};


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

impl Key<H256> for CurrentHash {
    type Target = H256;

    fn key(&self) -> H256 {
        H256::from("7cabfb7709b29c16d9e876e876c9988d03f9c3414e1d3ff77ec1de2d0ee59f66")
    }
}


fn main() {

    let matches = App::new("cita-recover")
        .version(get_build_info_str(true))
        .long_version(get_build_info_str(false))
        .author("yubo")
        .about("CITA Block Chain Node powered by Rust")
        .args_from_usage("-h, --height=[NUMBER] 'Sets the destinate height'")
        .args_from_usage("-d, --data=[PATH] 'Set data dir'")
        .get_matches();

    let data_path = matches.value_of("data").unwrap_or("./data");

    let mut db = DB::open_default(data_path);
    match db.get(b"7cabfb7709b29c16d9e876e876c9988d03f9c3414e1d3ff77ec1de2d0ee59f66") {
        Ok(Some(value)) => println!("retrieved value {}", value.to_utf8().unwrap()),
        Ok(None) => println!("value not found"),
        Err(e) => println!("operational problem encountered: {}", e),
    }


}
