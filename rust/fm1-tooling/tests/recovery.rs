// SPDX-License-Identifier: GPL-3.0-only
use fm1_tooling::{debugger, rescue, Adapter, Error};
use serde_json::{json, Value};
struct Offline;
impl Adapter for Offline {
    fn action(&mut self, _: Value) -> Result<Value, Error> {
        panic!("invalid recovery input performed I/O")
    }
}
#[test]
fn protected_head_and_upper_do_not_enter_plan() {
    let image = vec![0x55; rescue::APP_END];
    let mut now = vec![0x55; rescue::FLASH_SIZE];
    now[rescue::APP_END..].fill(0xa5);
    assert!(rescue::plan(&image, &now).unwrap().is_empty());
    for a in (rescue::WRITE_MIN..rescue::APP_END).step_by(rescue::SECTOR) {
        now[a] ^= 1;
    }
    let p = rescue::plan(&image, &now).unwrap();
    assert_eq!(p.len(), 143);
    assert_eq!(p[0], 0x4000);
    assert_eq!(p.last(), Some(&0x92000));
    now[0x3000] ^= 1;
    assert!(rescue::plan(&image, &now)
        .unwrap_err()
        .message
        .contains("protected head"));
}
#[test]
fn invalid_lengths_and_cdb_stop_before_io() {
    assert!(rescue::restore(&mut Offline, true, &[], &[], true, "", false).is_err());
    assert!(fm1_tooling::request(
        &json!({"method":"rescue.bot","args":[vec![0;17],null,0,5000]}),
        &mut Offline
    )
    .is_err());
    assert!(rescue::plan(&[], &[]).is_err());
}
#[test]
fn pinned_hash_uses_standard_sha256() {
    assert_eq!(
        rescue::sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        rescue::sha256(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert!(rescue::stock(b"abc", "stock").is_err());
    assert!(rescue::loader(b"abc").is_err());
}
#[test]
fn truncated_debug_report_retains_printed_prefix() {
    let p = debugger::plan("peek", 0x10000, 2).unwrap();
    let mut b = vec![0; 5];
    b.extend(fm1_formats::tool_wire::word35(0x12345678));
    b.extend([0; 4]);
    let r = debugger::report(&p, &b).unwrap();
    assert_eq!(r["text"], "00010000: 12345678\n");
    assert_eq!(r["error"]["kind"], "CodecError");
    assert_eq!(r["error"]["code"], "-1");
}
#[test]
fn zero_tick_debug_report_never_emits_nonfinite_output() {
    let p = debugger::plan("clock", 2000000, 0).unwrap();
    let r = debugger::report(&p, &[0; 15]).unwrap();
    assert_eq!(r["text"], "");
    assert_eq!(r["error"]["kind"], "ZeroDivisionError");
}
