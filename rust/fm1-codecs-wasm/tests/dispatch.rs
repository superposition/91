// SPDX-License-Identifier: GPL-3.0-only
use fm1_codecs_wasm::api;
use serde_json::Value;
fn raw(v: &Value) -> Vec<u8> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect()
}
fn request(v: &Value) -> Vec<u8> {
    let mut b = vec![0; 256];
    for (i, a) in v["args"].as_array().unwrap().iter().enumerate() {
        b[i * 4..i * 4 + 4].copy_from_slice(&(a.as_u64().unwrap() as u32).to_le_bytes());
    }
    for (i, d) in v["blobs"].as_array().unwrap().iter().enumerate() {
        let d = raw(d);
        let at = b.len() as u32;
        b[128 + i * 8..132 + i * 8].copy_from_slice(&at.to_le_bytes());
        b[132 + i * 8..136 + i * 8].copy_from_slice(&(d.len() as u32).to_le_bytes());
        b.extend(d);
    }
    b
}
#[test]
fn independent_c_js_python_vectors_execute_native_dispatch() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!("vectors.json")).unwrap();
    let mut output = vec![0xa5; 1_100_032];
    for v in vectors {
        let op = v["op"].as_u64().unwrap() as u32;
        let input = request(&v);
        let expected = raw(&v["expected"]);
        let n = api::run(op, &input, &mut output[..1_100_000])
            .unwrap_or_else(|e| panic!("{}: {e:?}", v["name"]));
        assert_eq!(&output[..n], expected, "{}", v["name"]);
        assert!(output[1_100_000..].iter().all(|v| *v == 0xa5));
    }
}
#[test]
fn bounded_decoder_mutations_and_hostile_descriptors_do_not_escape_windows() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!("vectors.json")).unwrap();
    let mut seed = 91u32;
    let mut output = vec![0xa5; 4128];
    let mut attempts = 0;
    for v in vectors.iter().step_by(7) {
        let input = request(v);
        let op = v["op"].as_u64().unwrap() as u32;
        for length in [
            0,
            1,
            127,
            255,
            256,
            input.len() / 2,
            input.len().saturating_sub(1),
            input.len(),
        ] {
            for cap in [0, 1, 17, 255, 4096] {
                output.fill(0xa5);
                let len = length.min(input.len());
                let result = api::run(op, &input[..len], &mut output[..cap]);
                if let Ok(n) = result {
                    assert!(n <= cap);
                }
                assert!(output[cap..].iter().all(|v| *v == 0xa5));
                attempts += 1;
            }
        }
        for _ in 0..32 {
            let mut mutated = input.clone();
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let index = seed as usize % mutated.len();
            mutated[index] ^= (seed >> 16) as u8;
            output.fill(0xa5);
            let cap = seed as usize % 4097;
            let result = api::run(op, &mutated, &mut output[..cap]);
            if let Ok(n) = result {
                assert!(n <= cap);
            }
            assert!(output[cap..].iter().all(|v| *v == 0xa5));
            attempts += 1;
        }
    }
    assert!(attempts > 20_000);
}
