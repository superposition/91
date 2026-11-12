// SPDX-License-Identifier: GPL-3.0-only
use fm1_formats::{containers::*, package, storage, voice, wire, CodecError};
use serde_json::Value;
fn c() -> Value {
    serde_json::from_str(include_str!("formats/c-reference.json")).unwrap()
}
fn js() -> Value {
    serde_json::from_str(include_str!("formats/js-reference.json")).unwrap()
}
fn py() -> Value {
    serde_json::from_str(include_str!("formats/python-reference.json")).unwrap()
}
fn bytes(v: &Value) -> Vec<u8> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect()
}
fn raw(n: usize, seed: usize, mul: usize) -> Vec<u8> {
    (0..n).map(|i| (seed + i * mul) as u8).collect()
}
fn hash(mut h: u32, data: &[u8]) -> u32 {
    for &b in data {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    h
}
fn record_hash(mut h: u32, result: &storage::DecodedRecord) -> u32 {
    h = hash(h, &[u8::from(result.valid)]);
    for p in result.values.parameters {
        h = hash(h, &p.to_le_bytes());
    }
    h = hash(h, &result.values.pool.to_le_bytes());
    for p in result.values.patterns {
        h = hash(h, &p.to_le_bytes());
    }
    h = hash(h, &result.values.modifiers);
    h = hash(h, &result.values.acid);
    hash(h, &result.values.pluck)
}
#[test]
fn dx7_conversion_matches_every_frozen_c_byte_and_field_mutation() {
    let c = c();
    for seed in 0..256 {
        let b: [u8; 128] = raw(128, seed, 37).try_into().unwrap();
        let v: [u8; 155] = raw(155, seed, 53).try_into().unwrap();
        assert_eq!(voice::unpack(&b).as_slice(), bytes(&c["voices"][seed][0]));
        assert_eq!(voice::pack(&v).as_slice(), bytes(&c["voices"][seed][1]));
    }
    for field in 0..283 {
        let mut h = 2166136261;
        for value in 0..256 {
            let mut b: [u8; 128] = raw(128, 91, 37).try_into().unwrap();
            let mut v: [u8; 155] = raw(155, 91, 53).try_into().unwrap();
            if field < 128 {
                b[field] = value as u8;
                h = hash(h, &voice::unpack(&b));
            } else {
                v[field - 128] = value as u8;
                h = hash(h, &voice::pack(&v));
            }
        }
        assert_eq!(
            h as u64,
            c["voice_mutations"][field].as_u64().unwrap(),
            "field{field}"
        );
    }
    for n in 0..160 {
        let b = vec![255; n];
        assert_eq!(voice::PackedVoice::parse(&b).is_ok(), n == 128);
        assert_eq!(voice::Voice::parse(&b).is_ok(), n == 155);
    }
    let b = [255; 128];
    assert_eq!(voice::PackedVoice::parse(&b).unwrap().bytes(), &b);
}
#[test]
fn stored_record_encoding_and_all_96256_byte_mutations_match_frozen_c() {
    let c = c();
    let factory: Value = serde_json::from_str(include_str!(
        "../../fm1-engine/tests/factory/reference.json"
    ))
    .unwrap();
    let mut recipe = [0; 176];
    for (i, p) in recipe.iter_mut().enumerate() {
        *p = factory["presets"][35]["parameters"][i].as_i64().unwrap() as i16;
    }
    for seed in 0..32 {
        let mut s = storage::RecordValues {
            parameters: [0; 176],
            pool: 0xab5,
            patterns: [0; 36],
            modifiers: raw(96, seed, 37).try_into().unwrap(),
            acid: raw(18, seed, 13).try_into().unwrap(),
            pluck: raw(12, seed, 17).try_into().unwrap(),
        };
        for (i, p) in s.parameters.iter_mut().enumerate() {
            *p = (seed as i16) * 19 - i as i16 * 3;
        }
        for (i, p) in s.patterns.iter_mut().enumerate() {
            *p = (seed * 907 + i * 1637) as u16;
        }
        let mut output = [0xa5; 378];
        assert_eq!(s.encode_legacy(&mut output[1..377]), Ok(376));
        assert_eq!(&output[1..377], bytes(&c["records"][seed]));
        assert_eq!(output[0], 0xa5);
        assert_eq!(output[377], 0xa5);
    }
    let mut b: [u8; 376] = raw(376, 91, 37).try_into().unwrap();
    b[..2].copy_from_slice(&0xab5u16.to_le_bytes());
    b[74..250].fill(255);
    for field in 0..376 {
        let old = b[field];
        let mut h = 2166136261;
        for value in 0..256 {
            b[field] = value as u8;
            let result = storage::Record::parse(&b)
                .unwrap()
                .decode_legacy(176, &recipe);
            h = record_hash(h, &result);
        }
        b[field] = old;
        assert_eq!(
            h as u64,
            c["record_mutations"][field].as_u64().unwrap(),
            "field{field}"
        );
    }
    for n in 0..=176 {
        let s = storage::Record::parse(&b)
            .unwrap()
            .decode(n, &recipe)
            .unwrap();
        assert_eq!(s.parameters, recipe);
    }
    b[0] = 0;
    assert!(storage::Record::parse(&b)
        .unwrap()
        .decode(176, &recipe)
        .is_err());
    assert!(storage::Record::parse(&b)
        .unwrap()
        .parameter(usize::MAX, 0, 176)
        .is_err());
    assert!(storage::Record::parse(&b[..375]).is_err());
}
#[test]
fn all_legacy_object_restore_mutations_and_lengths_match_frozen_c() {
    let c = c();
    let limits = storage::RestoreLimits {
        effects: c["effects"].as_u64().unwrap() as u16,
        settings_len: c["settings_len"].as_u64().unwrap() as usize,
    };
    for row in c["restore"].as_array().unwrap() {
        let object = row["object"].as_u64().unwrap() as u32;
        let mut b = bytes(&row["bytes"]);
        let n = b.len();
        let view = storage::ObjectView::parse(object, &b, limits).unwrap();
        let mut output = vec![0; n];
        view.encode(&mut output).unwrap();
        assert_eq!(output, b);
        match object {
            2 => {
                let settings = storage::Settings::parse(&b, limits).unwrap();
                assert_eq!(
                    settings.effects().len(),
                    fm1_formats::read_u16(&b, 4).unwrap() as usize
                );
                assert_eq!(settings.performance(), &[0; 12]);
                let _ = (
                    settings.pattern(),
                    settings.usb_flags(),
                    settings.targets_and_visualizer(),
                );
            }
            3 => {
                assert_eq!(view.voice(0).unwrap(), &[0; 128]);
                assert!(view.voice(16).is_err());
            }
            4 => {
                assert_eq!(view.set_list().unwrap().is_some(), n == 32);
            }
            23 | 25 => {
                assert!(view.extension(18, 9).unwrap().is_some());
                assert!(view.extension(19, 0).is_err());
            }
            24 => {
                assert_eq!(view.midi_entries().unwrap().count(), 3);
            }
            _ => {}
        }
        b.resize(n + 2, 0);
        let mut lh = 2166136261;
        for len in 0..=n + 2 {
            lh = hash(
                lh,
                &[u8::from(storage::restore_valid(object, &b[..len], limits))],
            );
        }
        assert_eq!(
            lh as u64,
            row["length_hash"].as_u64().unwrap(),
            "object{object}length"
        );
        let mut h = 2166136261;
        for field in 0..n {
            let old = b[field];
            for value in 0..256 {
                b[field] = value as u8;
                h = hash(
                    h,
                    &[u8::from(storage::restore_valid(object, &b[..n], limits))],
                );
            }
            b[field] = old;
        }
        assert_eq!(
            h as u64,
            row["mutation_hash"].as_u64().unwrap(),
            "object{object}mutations"
        );
    }
}

#[test]
fn legacy_signed_records_match_frozen_c_before_and_after_application_policy() {
    let c = c();
    for row in c["legacy"].as_array().unwrap() {
        let input = bytes(&row["input"]);
        let count = row["count"].as_u64().unwrap() as u16;
        let revision = row["revision"].as_u64().unwrap();
        let record = storage::LegacyRecord::parse(&input, count).unwrap();
        let decoded = record.decode(revision < 3, revision == 0);
        for (i, p) in decoded.parameters.iter().enumerate() {
            assert_eq!(*p as i64, row["prefix"][i].as_i64().unwrap());
        }
        assert_eq!(decoded.pool as u64, row["pool"].as_u64().unwrap());
        for (i, p) in decoded.patterns.iter().enumerate() {
            assert_eq!(*p as u64, row["patterns"][i].as_u64().unwrap());
        }
        assert_eq!(decoded.valid, row["saved"].as_u64().unwrap() != 0);
        assert!(storage::LegacyRecord::parse(&input[..input.len() - 1], count).is_err());
    }
    assert!(storage::LegacyRecord::parse(&[], 0).is_err());
    assert!(storage::LegacyRecord::parse(&[], 177).is_err());
    let mut kit = [0xa5; 132];
    kit[..4].copy_from_slice(b"HYP1");
    assert_eq!(storage::LegacyKit::parse(&kit).unwrap().theme(), 0xa5);
    assert!(storage::LegacyKit::parse(&kit[..131]).is_err());
    assert_eq!(voice::maximum(155), None);
    assert_eq!(voice::maximum(usize::MAX), None);
    let mut malformed = [0; 8];
    malformed[..4].copy_from_slice(b"SET1");
    malformed[6] = 12;
    assert!(storage::Settings::parse(
        &malformed,
        storage::RestoreLimits {
            effects: 0,
            settings_len: 8
        }
    )
    .is_err());
}
#[test]
fn piece_set_backup_files_match_original_javascript_and_preserve_unknown_bytes() {
    let js = js();
    for row in js["pieces"].as_array().unwrap() {
        let b = bytes(row);
        let p = PieceFile::parse(&b).unwrap();
        let mut output = vec![0xa5; b.len() + 2];
        p.encode(&mut output[1..b.len() + 1]).unwrap();
        assert_eq!(&output[1..b.len() + 1], b);
        let parts = PieceParts {
            genre: p.genre(),
            slot: p.slot(),
            flags: p.flags(),
            parameter_count: p.parameter_count(),
            record: p.record(),
            progression: p.progression(),
            voice: p.voice(),
            metadata: p.metadata(),
        };
        parts.encode(&mut output[1..b.len() + 1]).unwrap();
        assert_eq!(&output[1..b.len() + 1], b);
        assert_eq!(output[0], 0xa5);
        assert_eq!(output[b.len() + 1], 0xa5);
        for n in 0..b.len() {
            assert!(PieceFile::parse(&b[..n]).is_err());
        }
    }
    for row in js["sets"].as_array().unwrap() {
        let b = bytes(row);
        let s = SetFile::parse(&b).unwrap();
        let pieces: Vec<_> = (0..s.piece_count()).map(|i| s.piece(i).unwrap()).collect();
        let voices: Vec<_> = (0..s.voice_count()).map(|i| *s.voice(i).unwrap()).collect();
        let parts = SetParts {
            list: s.list(),
            play: s.play(),
            mix: s.mix(),
            pieces: &pieces,
            voices: &voices,
            metadata: s.metadata(),
        };
        let mut output = vec![0; b.len()];
        parts.encode(&mut output).unwrap();
        assert_eq!(output, b);
        assert!(s.piece(s.piece_count()).is_err());
        assert!(s.voice(s.voice_count()).is_err());
        for n in 0..b.len() {
            assert!(SetFile::parse(&b[..n]).is_err());
        }
    }
    for row in js["backups"].as_array().unwrap() {
        let b = bytes(row);
        let view = BackupFile::parse(&b).unwrap();
        let mut output = vec![0; b.len()];
        view.encode(&mut output).unwrap();
        assert_eq!(output, b);
        if view.trailing().is_empty() {
            let objects: Vec<_> = view.objects().collect();
            backup_encode(&objects, view.metadata(), &mut output).unwrap();
            assert_eq!(output, b);
        }
        for n in 0..b.len() {
            assert!(BackupFile::parse(&b[..n]).is_err());
        }
    }
}
#[test]
fn editor_backup_and_ota_framing_matches_independent_c_and_js() {
    let c = c();
    let js = js();
    let mut output = [0xa5; 640];
    for n in 0..=64 {
        let data = raw(n, n, 37);
        let len = wire::backup_frame(17, n as u16, 3000, &data, &mut output).unwrap();
        assert_eq!(&output[..len], bytes(&c["frames"][n]));
        let frame = wire::BackupData::parse_frame(&output[..len]).unwrap();
        let mut raw = [0; 64];
        assert_eq!(frame.decode(&mut raw).unwrap(), n);
        assert_eq!(&raw[..n], data);
    }
    let mut vals = [0i32; 24];
    for (i, v) in vals.iter_mut().enumerate() {
        *v = i as i32 * 9 - 100;
    }
    vals[23] = 8000;
    let n = wire::editor_dump(130, &vals, &mut output).unwrap();
    assert_eq!(&output[..n], bytes(&c["dump"]));
    let packed: [u8; 128] = raw(128, 91, 37).try_into().unwrap();
    let n = wire::editor_voice(15, 16, Some(&packed), &mut output).unwrap();
    assert_eq!(&output[..n], bytes(&c["editor_voice"]));
    for row in js["chunks"].as_array().unwrap() {
        let message = bytes(row);
        let raw_message = &message[1..message.len() - 1];
        let chunk = wire::BackupData::parse_chunk(raw_message).unwrap();
        let mut data = [0; 256];
        let n = chunk.decode(&mut data).unwrap();
        let len = wire::backup_chunk(
            chunk.object,
            chunk.index,
            chunk.total,
            &data[..n],
            &mut output,
        )
        .unwrap();
        assert_eq!(&output[..len], message);
        for end in 0..raw_message.len() {
            assert!(wire::BackupData::parse_chunk(&raw_message[..end]).is_err());
        }
    }
    for row in js["packed"].as_array().unwrap() {
        let input = bytes(&row["input"]);
        let encoded = bytes(&row["encoded"]);
        let n = wire::pack7(&input, &mut output).unwrap();
        assert_eq!(&output[..n], encoded);
        let mut decoded = [0; 512];
        let count = wire::unpack7(&encoded, &mut decoded).unwrap();
        assert_eq!(&decoded[..count], input);
    }
    for row in js["ota"].as_array().unwrap() {
        let message = bytes(row);
        let mut scratch = [0; 640];
        let decoded = wire::decode_ota(&message, &mut scratch).unwrap();
        let n = wire::encode_ota(decoded.command(), decoded.body(), &mut output).unwrap();
        assert_eq!(&output[..n], message);
        for n in 0..message.len() {
            assert!(wire::decode_ota(&message[..n], &mut scratch).is_err());
        }
    }
    let original = raw(3000, 91, 37);
    let mut restored = [0xa5; 3840];
    let mut session = wire::BackupSession::default();
    for index in 0..original.len().div_ceil(256) {
        let at = index * 256;
        let data = &original[at..(at + 256).min(original.len())];
        let n = wire::backup_chunk(23, index as u16, 3000, data, &mut output).unwrap();
        let chunk = wire::BackupData::parse_chunk(&output[1..n - 1]).unwrap();
        session.accept(chunk, &mut restored).unwrap();
        assert_eq!(
            session.complete(23, 3000),
            at + data.len() == original.len()
        );
    }
    assert_eq!(&restored[..3000], original);
    assert!(restored[3000..].iter().all(|v| *v == 0xa5));
}
#[test]
fn firmware_package_primitives_match_original_python_and_all_chip_keys() {
    let py = py();
    for seed in 0..256 {
        let input = raw(seed * 2, seed, 37);
        let row = &py["vectors"][seed];
        assert_eq!(
            package::crc16(&input, (seed * 257) as u16) as u64,
            row["crc"].as_u64().unwrap()
        );
        let mut encrypted = input.clone();
        package::enc(&mut encrypted, (seed * 257) as u16);
        assert_eq!(encrypted, bytes(&row["enc"]));
        let mut sf = input.clone();
        package::sfc(&mut sf, seed as u32, 0, 0x980f).unwrap();
        assert_eq!(sf, bytes(&row["sfc"]));
        assert_eq!(
            package::chip_key(&raw(32, seed, 37)).unwrap() as u64,
            row["key"].as_u64().unwrap()
        );
        assert_eq!(
            package::body_marker(&input).unwrap().as_slice(),
            bytes(&row["marker"])
        );
        package::validate_marker(&bytes(&row["marker"]), &input).unwrap();
    }
    let mut h = 2166136261;
    for key in 0..=u16::MAX {
        let blob = package::key_blob(key);
        assert_eq!(package::chip_key(&blob[..32]).unwrap(), key);
        h = hash(h, &blob);
    }
    assert_eq!(h as u64, py["key_blob_hash"].as_u64().unwrap());
    let b = bytes(&py["package"]);
    let package = package::Fwsc::parse(&b).unwrap();
    let mut identity = [0; 20];
    let n = package.product(&mut identity).unwrap();
    assert_eq!(&identity[..n], b"FM-1_7000300");
    let mut logical = vec![0; package.logical_len()];
    package.copy_logical(0, &mut logical).unwrap();
    let mut wrapped = vec![0; b.len()];
    package::wrap_logical(&logical, &identity[..n], &mut wrapped).unwrap();
    assert_eq!(wrapped, b);
    let directory = package.directory().unwrap();
    assert_eq!(directory.len(), 2);
    for index in 0..2 {
        directory.validate_data(index).unwrap();
        let entry = directory.entry(index).unwrap();
        let mut contents = vec![0; entry.len()];
        package.copy_logical(entry.offset(), &mut contents).unwrap();
        assert_eq!(
            contents,
            bytes(&py[if index == 0 { "flash" } else { "ota" }])
        );
    }
    assert!(directory.entry(2).is_err());
}
#[test]
fn bounded_mutation_decoders_reject_corruption_and_encoders_are_transactional() {
    let js = js();
    let limits = storage::RestoreLimits {
        effects: 512,
        settings_len: 544,
    };
    let mut rng = 91u32;
    let mut scratch = [0xa5; 8192];
    for pass in 0..20000 {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        let n = (rng as usize) % 4097;
        let mut b = raw(n, rng as usize, 37);
        if pass % 3 == 0 {
            let family = if pass % 2 == 0 { "pieces" } else { "sets" };
            b = bytes(&js[family][0]);
            let at = rng as usize % b.len();
            b[at] ^= (rng >> 8) as u8;
        }
        let _ = PieceFile::parse(&b);
        let _ = SetFile::parse(&b);
        let _ = BackupFile::parse(&b);
        let _ = storage::Bank::parse(&b, (rng % 20) as u8);
        let _ = storage::restore_valid(rng % 32, &b, limits);
        let _ = wire::BackupData::parse_chunk(&b);
        let _ = wire::decode_ota(&b, &mut scratch);
        if let Ok(p) = package::Fwsc::parse(&b) {
            let _ = p.directory();
        }
        if let Ok(v) = voice::PackedVoice::parse(&b) {
            let _ = v.unpack();
        }
    }
    let p = bytes(&js["pieces"][0]);
    let view = PieceFile::parse(&p).unwrap();
    let mut guarded = [0xa5; 400];
    assert_eq!(view.encode(&mut guarded[..10]), Err(CodecError::Capacity));
    assert_eq!(guarded, [0xa5; 400]);
    let mut corrupt = p.clone();
    corrupt[5] = 19;
    let len = corrupt.len();
    let crc = fm1_formats::crc32(&corrupt[..len - 4]);
    corrupt[len - 4..].copy_from_slice(&crc.to_le_bytes());
    assert!(PieceFile::parse(&corrupt).is_err());
    assert!(PieceFile::parse_compatible(&corrupt).is_ok());
    let mut bad_nibbles = [0xa5; 32];
    assert_eq!(
        wire::decode_nibbles(&[0, 1, 16, 2], &mut bad_nibbles),
        Err(CodecError::Nibble)
    );
    assert_eq!(bad_nibbles, [0xa5; 32]);
    assert!(wire::editor_dump(0, &[0; 25], &mut bad_nibbles).is_err());
    assert_eq!(bad_nibbles, [0xa5; 32]);
    assert!(package::sfc(&mut bad_nibbles, 0, 1, 0).is_err());
    assert_eq!(bad_nibbles, [0xa5; 32]);
}

#[test]
fn jlfs_headers_and_compensation_match_the_original_package_builder() {
    let py = py();
    for row in py["entries"].as_array().unwrap() {
        let bytes = bytes(row);
        let entry = package::JlfsEntry::parse(&bytes).unwrap();
        let parts = package::JlfsParts {
            data_checksum: entry.data_checksum(),
            offset: entry.offset(),
            length: entry.len(),
            flags: entry.flags(),
            reserved: entry.reserved(),
            index: entry.index(),
            raw_name: entry.raw_name(),
        };
        assert_eq!(parts.encode().as_slice(), bytes);
        let mut bad = bytes.clone();
        bad[13] ^= 1;
        assert!(package::JlfsEntry::parse(&bad).is_err());
        for end in 0..32 {
            assert!(package::JlfsEntry::parse(&bytes[..end]).is_err());
        }
    }
    for row in py["compensation"].as_array().unwrap() {
        let mut app = bytes(&row["app"]);
        let offset = row["offset"].as_u64().unwrap() as usize;
        let want = row["want"].as_u64().unwrap() as u16;
        let correction = package::compensate(&app, offset, want).unwrap();
        assert_eq!(correction.as_slice(), bytes(&row["bytes"]));
        app[offset..offset + 4].copy_from_slice(&correction);
        assert_eq!(package::crc16(&app, 0), want);
    }
    assert!(package::compensate(&[0; 16], usize::MAX, 0).is_err());
    assert!(package::compensate(&[1; 16], 0, 0).is_err());
}

#[test]
fn dx7_sysex_and_malformed_classification_match_original_usb_source() {
    let c = c();
    for row in c["dx"].as_array().unwrap() {
        let message = bytes(&row["bytes"]);
        let inner = &message[1..message.len() - 1];
        let mut padded = inner.to_vec();
        padded.extend_from_slice(&[0, 0]);
        let mut h = 2166136261;
        for n in 0..=inner.len() + 2 {
            h = hash(
                h,
                &[voice::classify_legacy(&padded[..n]).map_or(0, |k| k as u8)],
            );
        }
        assert_eq!(h as u64, row["length_hash"].as_u64().unwrap());
        let mut data = inner.to_vec();
        let mut h = 2166136261;
        for field in 0..8.min(data.len()) {
            let old = data[field];
            for value in 0..128 {
                data[field] = value;
                h = hash(h, &[voice::classify_legacy(&data).map_or(0, |k| k as u8)]);
            }
            data[field] = old;
        }
        assert_eq!(h as u64, row["header_hash"].as_u64().unwrap());
        let view = voice::DxMessage::parse(&message).unwrap();
        assert_eq!(view.kind() as u64, row["kind"].as_u64().unwrap());
        let mut output = vec![0; message.len()];
        view.encode(&mut output).unwrap();
        assert_eq!(output, message);
        let n = match view.kind() {
            voice::IncomingKind::Bulk => {
                let payload: &[u8; 4096] = message[6..4102].try_into().unwrap();
                assert!(view.packed(32).is_err());
                voice::dx7_bulk(view.channel().unwrap(), payload, &mut output).unwrap()
            }
            voice::IncomingKind::Single => {
                voice::dx7_single(view.channel().unwrap(), view.single().unwrap(), &mut output)
                    .unwrap()
            }
            voice::IncomingKind::Slot => voice::slot_voice(
                view.slot().unwrap(),
                Some(view.packed(0).unwrap()),
                &mut output,
            )
            .unwrap(),
            voice::IncomingKind::Clear => voice::slot_voice(0, None, &mut output).unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(&output[..n], message);
        for end in 0..message.len() {
            assert!(voice::DxMessage::parse(&message[..end]).is_err());
        }
        if view.kind() != voice::IncomingKind::Clear {
            let mut bad = message.clone();
            let last = bad.len() - 2;
            bad[last] ^= 1;
            assert!(matches!(
                voice::DxMessage::parse(&bad),
                Err(CodecError::Checksum)
            ));
        }
    }
}

#[test]
fn typed_reply_views_keep_original_header_fields_and_reject_bad_checksums() {
    let c = c();
    let dump = bytes(&c["dump"]);
    let view = wire::EditorDump::parse(&dump).unwrap();
    assert_eq!(view.first(), 130);
    assert_eq!(view.len(), 24);
    assert_eq!(view.value(0).unwrap(), -100);
    assert_eq!(view.value(23).unwrap(), 8000);
    assert!(view.value(24).is_err());
    for n in 0..dump.len() {
        assert!(wire::EditorDump::parse(&dump[..n]).is_err());
    }
    let voice = bytes(&c["editor_voice"]);
    let view = wire::VoiceReply::parse(&voice).unwrap();
    assert_eq!(view.slot(), 15);
    assert_eq!(view.loaded(), 16);
    assert!(view.voice().is_some());
    let mut bad = voice.clone();
    bad[134] ^= 1;
    assert!(matches!(
        wire::VoiceReply::parse(&bad),
        Err(CodecError::Checksum)
    ));
    let mut output = [0; 160];
    let n = wire::editor_voice(20, 16, None, &mut output).unwrap();
    assert!(wire::VoiceReply::parse(&output[..n])
        .unwrap()
        .voice()
        .is_none());
    let n = wire::backup_ack(3, 127, 2, &mut output).unwrap();
    assert_eq!(&output[..n], &[240, 125, 80, 68, 3, 127, 2, 247]);
    assert_eq!(
        wire::ControlReply::parse(&output[..n]).unwrap(),
        wire::ControlReply::BackupAck {
            object: 3,
            index: 127,
            status: 2
        }
    );
    let n = wire::about(3, 176, 19, 5, &mut output).unwrap();
    assert_eq!(&output[..n], &[240, 125, 80, 69, 3, 1, 48, 19, 5, 247]);
    assert_eq!(
        wire::ControlReply::parse(&output[..n]).unwrap(),
        wire::ControlReply::About {
            version: 3,
            parameters: 176,
            genres: 19,
            voices: 5
        }
    );
    output[n - 1] = 0;
    assert!(wire::ControlReply::parse(&output[..n]).is_err());
    let js = js();
    let message = bytes(&js["jam_request"]);
    let req = wire::JamRequest::parse(&message[1..message.len() - 1]).unwrap();
    assert_eq!(
        (req.operation(), req.token(), req.payload()),
        (1, 0, &[][..])
    );
    let data = [1, 0, 15, 4, 26, 30];
    let n = wire::jam_reply(1, 0, 0, &data, &mut output).unwrap();
    let reply = wire::JamReply::parse(&output[..n]).unwrap();
    assert_eq!(
        (reply.operation(), reply.token(), reply.status()),
        (1, 0, 0)
    );
    let mut decoded = [0; 68];
    assert_eq!(reply.decode(&mut decoded).unwrap(), 6);
    assert_eq!(&decoded[..6], data);
    output[n - 2] ^= 1;
    assert!(matches!(
        wire::JamReply::parse(&output[..n]),
        Err(CodecError::Checksum)
    ));
}
