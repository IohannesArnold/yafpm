use std::str::FromStr;
use std::collections::HashMap;

use yafpm::{BuildCxt,Resource,Package};
use url::Url;
use blake2::Blake2s;
use digest::Digest;
use digest::generic_array::GenericArray;
use data_encoding::HEXLOWER;

fn dep_build_test() {
    let elfify_full_url = concat!(
        "file://",
        env!("CARGO_MANIFEST_DIR"),
        "/tests/pkgs/elfify.x");
    let elfify_bytes = include_bytes!("pkgs/elfify.x");
    let elfify_hash = Blake2s::digest(elfify_bytes);
    let elfify = Resource::new(
        "elfify.x",
        elfify_hash.into(),
        Url::from_str(elfify_full_url).unwrap()
    );

    let unhex_hash = HEXLOWER.decode(
        b"26f175461396f1cb925805416d6eb75dc867357764457ccb9a8488b0a6e86bc6"
    ).unwrap();
    let unhex = Package::new(
        "unhex",
        "0.0",
        GenericArray::clone_from_slice(&unhex_hash).into()
    );

    let temp_dir = std::env::temp_dir();
    let mut cxt = BuildCxt::new(
        "elfify",
        "0.0",
        GenericArray::clone_from_slice(&[29, 40, 20, 50, 228, 172, 136, 181,
        165, 76, 143, 147, 152, 22, 137, 122, 15, 37, 132, 36, 249, 240, 18,
        8, 250, 216, 171, 86, 55, 247, 244, 47]).into(),
"/tmp/unhex-0.0-E3YXKRQTS3Y4XESYAVAW23VXLXEGONLXMRCXZS42QSELBJXINPDA/unhex",
        HashMap::new(),
    );
    cxt.add_srcs([elfify]).add_build_deps([unhex]).add_build_cmd_args([
        "/elfify.x",
"/tmp/elfify-0.0-DUUBIMXEVSELLJKMR6JZQFUJPIHSLBBE7HYBECH23CVVMN7X6QXQ/elfify"
    ]);
    let out_dir = temp_dir.join(cxt.pkg_info.pkg_ident());
    cxt.exec_build(temp_dir.as_os_str()).unwrap();
    assert!(out_dir.exists());
}

fn main() {
    println!();
    print!("test test_mount::dep_build_test ... ");
    dep_build_test();
    println!("ok");
}
