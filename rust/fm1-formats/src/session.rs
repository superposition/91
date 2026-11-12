// SPDX-License-Identifier: GPL-3.0-only
//! Legacy 20-byte Songbook sessions, including untouched extension bytes.
use fm1_types::JamSettings;

pub const SESSION_LEN: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    Length,
    Header,
    Settings,
    Tempo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JamSession {
    pub settings: JamSettings,
    pub tempo: u16,
    /// Legacy readers accept all four bytes. Preserve unknown values rather
    /// than interpreting reserved fields as additional validity requirements.
    pub trailer: [u8; 4],
}

impl JamSession {
    pub fn decode(bytes: &[u8]) -> Result<Self, SessionError> {
        if bytes.len() != SESSION_LEN {
            return Err(SessionError::Length);
        }
        if &bytes[..4] != b"JSM\x01" {
            return Err(SessionError::Header);
        }
        let session = Self {
            settings: JamSettings {
                on: bytes[4],
                key: bytes[5],
                minor: bytes[6],
                bars: bytes[7],
                form: bytes[8],
                movement: bytes[9],
                energy: bytes[10],
                hold: bytes[11],
                bass: bytes[12],
                slot: bytes[13],
            },
            tempo: u16::from_le_bytes([bytes[14], bytes[15]]),
            trailer: [bytes[16], bytes[17], bytes[18], bytes[19]],
        };
        session.validate()?;
        Ok(session)
    }

    fn validate(&self) -> Result<(), SessionError> {
        self.settings
            .validate()
            .map_err(|_| SessionError::Settings)?;
        if !(70..=190).contains(&self.tempo) {
            return Err(SessionError::Tempo);
        }
        Ok(())
    }

    /// Write exactly twenty bytes; leave a larger caller buffer's tail alone.
    /// Invalid settings or insufficient space leave the whole buffer untouched.
    pub fn encode_into(&self, output: &mut [u8]) -> Result<usize, SessionError> {
        if output.len() < SESSION_LEN {
            return Err(SessionError::Length);
        }
        self.validate()?;
        let s = &self.settings;
        output[..4].copy_from_slice(b"JSM\x01");
        output[4..14].copy_from_slice(&[
            s.on, s.key, s.minor, s.bars, s.form, s.movement, s.energy, s.hold, s.bass, s.slot,
        ]);
        output[14..16].copy_from_slice(&self.tempo.to_le_bytes());
        output[16..20].copy_from_slice(&self.trailer);
        Ok(SESSION_LEN)
    }

    /// The optional factory source hint applies only when its marker is present.
    /// A saved slot still takes precedence when the application restores it.
    pub fn builtin_source_hint(&self) -> Option<u8> {
        (self.trailer[1] == b'S' && self.trailer[0] < 3).then_some(self.trailer[0])
    }
}
