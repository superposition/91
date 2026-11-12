// SPDX-License-Identifier: GPL-3.0-only
use fm1_formats::session::{JamSession, SessionError, SESSION_LEN};

const DEFAULT: [u8; SESSION_LEN] = [
    b'J', b'S', b'M', 1, 1, 0, 0, 16, 0, 3, 60, 0, 0, 0, 100, 0, 0, 0, 0, 0,
];

fn byte(h: u32, value: bool) -> u32 {
    (h ^ u32::from(value)).wrapping_mul(16_777_619)
}

#[test]
fn legacy_c_session_validity_and_every_accepted_mutation_round_trip() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("session-reference.json")).unwrap();
    let mut data = [0; SESSION_LEN + 2];
    data[..SESSION_LEN].copy_from_slice(&DEFAULT);
    let mut lengths = 2_166_136_261;
    for length in 0..=data.len() {
        lengths = byte(lengths, JamSession::decode(&data[..length]).is_ok());
    }
    let mut h = 2_166_136_261;
    let mut accepted = 0;
    for index in 0..SESSION_LEN {
        let original = data[index];
        for value in 0..=255 {
            data[index] = value;
            let decoded = JamSession::decode(&data[..SESSION_LEN]);
            h = byte(h, decoded.is_ok());
            if let Ok(session) = decoded {
                accepted += 1;
                let mut output = [0xaa; SESSION_LEN + 2];
                assert_eq!(session.encode_into(&mut output).unwrap(), SESSION_LEN);
                assert_eq!(output[..SESSION_LEN], data[..SESSION_LEN]);
                assert_eq!(output[SESSION_LEN..], [0xaa, 0xaa]);
            }
        }
        data[index] = original;
    }
    let mut tempos = 2_166_136_261;
    let mut tempo_accepted = 0;
    for tempo in 0..=u16::MAX {
        data[14..16].copy_from_slice(&tempo.to_le_bytes());
        let valid = JamSession::decode(&data[..SESSION_LEN]).is_ok();
        tempos = byte(tempos, valid);
        tempo_accepted += u32::from(valid);
    }
    assert_eq!(h, reference["mutation_hash"].as_u64().unwrap() as u32);
    assert_eq!(accepted, reference["mutation_accepted"].as_u64().unwrap());
    assert_eq!(lengths, reference["length_hash"].as_u64().unwrap() as u32);
    assert_eq!(tempos, reference["tempo_hash"].as_u64().unwrap() as u32);
    assert_eq!(
        tempo_accepted,
        reference["tempo_accepted"].as_u64().unwrap() as u32
    );
}

#[test]
fn invalid_encoding_is_transactional_and_legacy_hints_remain_optional() {
    let mut session = JamSession::decode(&DEFAULT).unwrap();
    let mut output = [0xa5; SESSION_LEN];
    session.tempo = 69;
    assert_eq!(session.encode_into(&mut output), Err(SessionError::Tempo));
    assert_eq!(output, [0xa5; SESSION_LEN]);
    session.tempo = 190;
    session.settings.key = 12;
    assert_eq!(
        session.encode_into(&mut output),
        Err(SessionError::Settings)
    );
    assert_eq!(output, [0xa5; SESSION_LEN]);
    session.settings.key = 11;
    assert_eq!(
        session.encode_into(&mut output[..SESSION_LEN - 1]),
        Err(SessionError::Length)
    );
    assert_eq!(output, [0xa5; SESSION_LEN]);
    assert_eq!(session.builtin_source_hint(), None);
    session.trailer = [2, b'S', 255, 255];
    assert_eq!(session.builtin_source_hint(), Some(2));
    session.encode_into(&mut output).unwrap();
    assert_eq!(JamSession::decode(&output).unwrap(), session);
    session.trailer[0] = 3;
    assert_eq!(session.builtin_source_hint(), None);
}
