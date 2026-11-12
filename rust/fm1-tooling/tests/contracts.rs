// SPDX-License-Identifier: GPL-3.0-only
use fm1_tooling::{installer, summary, telemetry, Adapter, Error};
use serde_json::{json, Value};
struct Offline;
impl Adapter for Offline {
    fn action(&mut self, _: Value) -> Result<Value, Error> {
        panic!("offline request performed I/O")
    }
}
#[test]
fn identity_frame_and_decimal_limits() {
    let mut raw = [0; 34];
    raw[..3].copy_from_slice(&[0, 89, 17]);
    let name = b"A_999999999999999999999999";
    raw[6..6 + name.len()].copy_from_slice(name);
    let mut packet = [0; 42];
    packet[0] = 240;
    let n = fm1_formats::wire::pack7(&raw, &mut packet[1..]).unwrap();
    packet[n + 1] = 247;
    let packet = &packet[..n + 2];
    let id = installer::identity(packet).unwrap();
    assert_eq!(id["version"], "999999999999999999999999");
    let mut bad = packet.to_vec();
    bad[0] = 0;
    assert!(installer::identity(&bad).is_none());
    bad[0] = 240;
    *bad.last_mut().unwrap() = 0;
    assert!(installer::identity(&bad).is_none());
}
#[test]
fn csv_quotes_and_bounded_malformed_input() {
    assert_eq!(summary::csv(b"event\n\"\"").unwrap()[0]["event"], "");
    let rows = summary::csv(b"t,event,cpu\r\n0.25,\"a, b\"\"c\nnext\",31\r\n").unwrap();
    assert_eq!(rows[0]["event"], "a, b\"c\nnext");
    assert!(summary::csv(b"a,b\n1\n").is_err());
    assert!(summary::csv(b"a\n\"open").is_err());
    assert!(summary::csv(&[255]).is_err());
}
#[test]
fn events_keep_signed_sequence_reset_and_order() {
    let before = json!({"seq":4294967295u32,"xruns":1,"late":1,"stores":1,"shed":0,"crashes":1});
    let after = json!({"seq":0,"xruns":3,"late":4,"stores":2,"store_us":9,"shed":2,"crashes":2,"max_us":12,"budget_us":10,"gap_us":21001});
    assert_eq!(
        telemetry::events(&after, &before),
        [
            "XRUN+2",
            "LATE+3",
            "FLASH(9us)",
            "LOST-4294967296",
            "SHED2",
            "CRASH",
            "OVER",
            "GAP21ms"
        ]
    );
}
#[test]
fn bench_zero_divisor_preserves_printed_prefix() {
    let mut w = vec![json!(10); 16];
    w[0] = json!(0);
    w[1] = json!(24);
    let report = telemetry::bench(&w).unwrap();
    assert_eq!(
        report["text"],
        "bench:0 turns each,24 ticks per us\n"
            .replace("bench:0", "bench: 0")
            .replace("each,24", "each, 24")
    );
    assert_eq!(report["error"], "division by zero");
    w[0] = json!(32);
    let report = telemetry::bench(&w).unwrap();
    assert_eq!(report["text"].as_str().unwrap().lines().count(), 9);
}
#[test]
fn malformed_offline_api_requests_never_touch_adapter() {
    for v in [
        json!({"method":"install.pair","args":[]}),
        json!({"method":"telemetry.decode","args":[999,[]]}),
        json!({"method":"telemetry.summary","args":["bad",[]]}),
        json!({"method":"unknown"}),
    ] {
        assert!(fm1_tooling::request(&v, &mut Offline).is_err());
    }
    assert!(fm1_tooling::bytes(&json!([256])).is_err());
    assert!(fm1_tooling::bytes(&json!([-1])).is_err());
}
