// SPDX-License-Identifier: GPL-3.0-only
#[path = "support/bus.rs"]
mod bus;
#[test]
fn exact_frozen_register_width_order_reads_and_writes() {
    for row in include_str!("registers-golden.jsonl").lines() {
        let v: serde_json::Value = serde_json::from_str(row).unwrap();
        let operation = v["operation"].as_u64().unwrap() as u32;
        let variant = v["variant"].as_u64().unwrap() as u32;
        let mut io = bus::Bus::new(variant);
        let result = bus::execute(&mut io, operation, variant);
        io.word(result);
        assert_eq!(v["result"], result, "op{operation}/variant{variant}");
        assert_eq!(v["events"], io.events, "op{operation}/variant{variant}");
        assert_eq!(
            v["hash"],
            format!("{:08x}", io.hash),
            "op{operation}/variant{variant}"
        );
    }
}
#[test]
fn invalid_irq_does_not_issue_register_access() {
    let mut io = bus::Bus::new(0);
    assert!(!fm1_hal::drivers::irq_attach(&mut io, 128, 0x02001000, 4));
    assert!(!fm1_hal::drivers::irq_attach(
        &mut io,
        u32::MAX,
        0x02001000,
        4
    ));
    assert_eq!(io.events, 0);
}
