use std::str::FromStr;
use std::collections::HashMap;

use yafpm::{BuildCxt,Resource,Package};
use url::Url;
use blake2::Blake2s;
use digest::Digest;
use digest::generic_array::GenericArray;
use data_encoding::HEXLOWER;

fn basic_build_test() {
    let bin_full_url = concat!(
        "file://",
        env!("CARGO_MANIFEST_DIR"),
        "/tests/pkgs/unhex");
    let bin_bytes = include_bytes!("pkgs/unhex");
    let bin_hash = Blake2s::digest(bin_bytes);
    let bin = Resource::new(
        "unhex",
        bin_hash.into(),
        Url::from_str(bin_full_url).unwrap()
    );

    let hex_full_url = concat!(
        "file://",
        env!("CARGO_MANIFEST_DIR"),
        "/tests/pkgs/unhex.x");
    let hex_bytes = include_bytes!("pkgs/unhex.x");
    let hex_hash = Blake2s::digest(hex_bytes);
    let hex = Resource::new(
        "unhex.x",
        hex_hash.into(),
        Url::from_str(hex_full_url).unwrap()
    );

    let temp_dir = std::env::temp_dir();
    let output_hash = HEXLOWER.decode(
        b"26f175461396f1cb925805416d6eb75dc867357764457ccb9a8488b0a6e86bc6"
    ).unwrap();
    let mut cxt = BuildCxt::new(
        "unhex",
        "0.0",
        GenericArray::clone_from_slice(&output_hash).into(),
        "/unhex",
        HashMap::new(),
    );
    cxt.add_srcs([bin, hex]).add_build_cmd_args([
        "/unhex.x",
"/tmp/unhex-0.0-E3YXKRQTS3Y4XESYAVAW23VXLXEGONLXMRCXZS42QSELBJXINPDA/unhex"
    ]);
    let out_dir = temp_dir.join(cxt.pkg_info.pkg_ident());
    cxt.exec_build(temp_dir.as_os_str()).unwrap();
    assert!(out_dir.exists());
}

fn main() {
    println!();
    print!("test test_mount::basic_build_test ... ");
    basic_build_test();
    println!("ok");
}
