// SPDX-License-Identifier: GPL-3.0-only
//! Deterministic preview scenario and process/file orchestration. The actual
//! audio engine remains the maintained native simulator, never a resynthesizer.
use crate::{get, integer, midi::round_even, Error, Limits, Result};
use serde_json::{json, Value};
#[cfg(windows)]
use std::path::Path;
use std::{fs, path::PathBuf, process::Command};
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub bars: u8,
    pub bpm: u16,
    pub key: u8,
    pub minor: bool,
    pub phrases: u8,
}
impl Options {
    pub(crate) fn from_json(v: &Value) -> Result<Self> {
        Ok(Self {
            bars: integer(get(v, "bars")?, 0, 255, "bars")? as u8,
            bpm: integer(get(v, "bpm")?, 0, 65535, "BPM")? as u16,
            key: integer(get(v, "key")?, 0, 11, "key")? as u8,
            minor: v["minor"].as_bool().unwrap_or(false),
            phrases: integer(get(v, "phrases")?, 0, 255, "phrases")? as u8,
        })
    }
}
pub fn plan(o: Options) -> Result<String> {
    if ![8, 16].contains(&o.bars) {
        return Err(Error::value("bars must be 8 or 16"));
    }
    if !(70..=190).contains(&o.bpm) || !(1..=16).contains(&o.phrases) {
        return Err(Error::value("BPM 70..190; phrases 1..16"));
    }
    if o.key > 11 {
        return Err(Error::value("key must be 0..11"));
    }
    let duration = round_even(o.bars as u64 * 4 * 60000 * o.phrases as u64, o.bpm as u64);
    Ok(format!("jamstop\njampack preview.jmp\njam 1 {}\njam 2 {}\nsetpar 5 {}\njam 0 1\nwait 60\nwav preview.wav\ntap PLAY\nwait {duration}\njamstop\nwait 2000\nwavstop\nshot preview.png\n",o.key+12*u8::from(o.minor),o.bars,o.bpm))
}
fn checked(command: &mut Command) -> Result<()> {
    // API stdout contains only its JSON response. The child simulator's normal
    // diagnostics are forwarded to stderr, preserving failures and artifacts.
    let result = command.output()?;
    use std::io::Write;
    std::io::stderr().write_all(&result.stdout)?;
    std::io::stderr().write_all(&result.stderr)?;
    if !result.status.success() {
        return Err(Error {
            kind: "RuntimeError",
            message: format!("preview process failed: {}", result.status),
        });
    }
    Ok(())
}
#[cfg(windows)]
fn linux(path: &Path) -> Result<String> {
    let value = path.to_string_lossy().replace('\\', "/");
    let value = value.strip_prefix("//?/").unwrap_or(&value);
    let b = value.as_bytes();
    if b.len() < 3 || b[1] != b':' {
        return Err(Error::value("preview requires a local Windows drive"));
    }
    Ok(format!(
        "/mnt/{}{}",
        char::from(b[0]).to_ascii_lowercase(),
        &value[2..]
    ))
}
pub fn run(
    root: &str,
    pack: &str,
    output: &str,
    options: Options,
    limits: Limits<'_>,
) -> Result<Value> {
    let script = plan(options)?;
    let output = std::path::absolute(output)?;
    fs::create_dir_all(&output)?;
    fs::write(output.join("preview.jmp"), crate::cli::load(pack, limits)?)?;
    // Python text mode used the platform newline convention for this scenario.
    fs::write(
        output.join("preview.amb"),
        if cfg!(windows) {
            script.replace('\n', "\r\n")
        } else {
            script
        },
    )?;
    let root = std::path::absolute(root)?;
    let engine = root.join("build/host/amb_host");
    #[cfg(not(windows))]
    {
        checked(
            Command::new("sh")
                .arg("host/build_host.sh")
                .current_dir(&root)
                .env_remove("AMB_LEGACY_BOOT"),
        )?;
        checked(
            Command::new(&engine)
                .args(["preview.amb", "."])
                .current_dir(&output)
                .env_remove("AMB_LEGACY_BOOT"),
        )?;
    }
    #[cfg(windows)]
    {
        checked(
            Command::new("wsl")
                .args([
                    "-d",
                    "Ubuntu-22.04",
                    "--cd",
                    &linux(&root)?,
                    "--",
                    "bash",
                    "-lc",
                    "exec sh host/build_host.sh",
                ])
                .env_remove("AMB_LEGACY_BOOT"),
        )?;
        checked(
            Command::new("wsl")
                .args([
                    "-d",
                    "Ubuntu-22.04",
                    "--cd",
                    &linux(&output)?,
                    "--",
                    &linux(&engine)?,
                    "preview.amb",
                    ".",
                ])
                .env_remove("AMB_LEGACY_BOOT"),
        )?;
    }
    fs::write(
        output.join("preview.mid"),
        crate::midi_from_log(
            &fs::read_to_string(output.join("midi_out.txt"))?,
            options.bpm as u32,
        )?,
    )?;
    let path = |name: &str| -> PathBuf { output.join(name) };
    Ok(json!({"wav":path("preview.wav"),"midi":path("preview.mid"),"screen":path("preview.png")}))
}
