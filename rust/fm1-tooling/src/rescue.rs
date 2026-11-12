// SPDX-License-Identifier: GPL-3.0-only
//! ROM recovery commands and bounded flash restoration policy. This module has
//! no USB/file/network handles; only the supplied synchronous adapter performs
//! I/O. The protected head and upper objects never enter a write operation.
use crate::{bytes, number, text, Adapter, Error, Result};
use fm1_formats::{package, tool_wire, CodecError};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
pub const STOCK_SHA256: &str = "db1642b2b6fa5c2cccb11ffd13878068bb28601678d3644049f99dc40e7edb8a";
pub const LOADER_SHA256: &str = "d41da6126760c9d66660bcc0cac8d27d221806c5e369a8036921efe68dca5376";
pub const LOADER_URL:&str="https://raw.githubusercontent.com/kagaimiq/jl-uboot-tool/c32b28e7a04f873f526ba929cb1bae9f05442fe9/data/loaderblobs/usb/wl82loader.bin";
pub const FLASH_SIZE: usize = 0x100000;
pub const APP_END: usize = 0x93000;
pub const WRITE_MIN: usize = 0x4000;
pub const SECTOR: usize = 0x1000;
pub const IO: usize = 512;
const LOADER_ADDR: u32 = 0x1c02000;
const WRITE_MEMORY: u16 = 0xfb06;
const JUMP: u16 = 0xfb08;
const ERASE: u16 = 0xfb01;
const WRITE_FLASH: u16 = 0xfb04;
const READ_FLASH: u16 = 0xfd05;
const READ_KEY: u16 = 0xfc09;
const ONLINE: u16 = 0xfc0a;
const RUN_APP: u16 = 0xfc0c;
pub fn fail(message: impl Into<String>) -> Error {
    Error {
        kind: "Fail".into(),
        code: String::new(),
        message: message.into(),
    }
}
fn structure(message: impl Into<String>) -> Error {
    Error {
        kind: "StructError".into(),
        code: String::new(),
        message: message.into(),
    }
}
pub fn sha256(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn config() -> Value {
    json!({"vid":0x4c4a,"pid":0x8057,"stock_sha256":STOCK_SHA256,"loader_sha256":LOADER_SHA256,"loader_url":LOADER_URL,"loader_addr":LOADER_ADDR,"loader_arg":1,"flash_ids":[0x856014,0xb36014],"flash_size":FLASH_SIZE,"app_end":APP_END,"write_min":WRITE_MIN,"sector":SECTOR,"io":IO,"backend_paths":[null,"/opt/homebrew/lib/libusb-1.0.dylib","/usr/local/lib/libusb-1.0.dylib"]})
}
pub fn flash_image(raw: &[u8]) -> Result<Vec<u8>> {
    let mut out = vec![0; 1_100_000];
    let n = tool_wire::rescue_flash_compatible(raw, &mut out).map_err(|e| {
        fail(match e {
            CodecError::Index => "package has no flash entry",
            CodecError::Checksum => "package flash entry: bad CRC",
            _ => "not an FM-1 package (header)",
        })
    })?;
    out.truncate(n);
    Ok(out)
}
pub fn stock(raw: &[u8], path: &str) -> Result<Vec<u8>> {
    if sha256(raw) != STOCK_SHA256 {
        return Err(fail(format!(
            "{path} is not M-VAVE's FM-1 V15 package (sha256 mismatch)"
        )));
    }
    let image = flash_image(raw)?;
    if image.len() != APP_END {
        return Err(fail("unexpected flash entry size"));
    }
    Ok(image)
}
pub fn loader(blob: &[u8]) -> Result<()> {
    if sha256(blob) != LOADER_SHA256 {
        return Err(fail("wl82loader.bin: unexpected contents"));
    }
    Ok(())
}
pub fn cdb(cmd: u16, args: &[u8]) -> Result<Vec<u8>> {
    if args.len() > 14 {
        return Err(fail("command arguments exceed14 bytes"));
    }
    let mut v = vec![255; 16];
    v[..2].copy_from_slice(&cmd.to_be_bytes());
    v[2..2 + args.len()].copy_from_slice(args);
    Ok(v)
}
pub fn plan(image: &[u8], now: &[u8]) -> Result<Vec<usize>> {
    if image.len() != APP_END || now.len() != FLASH_SIZE {
        return Err(fail("unexpected flash snapshot size"));
    }
    let diff: Vec<_> = (0..APP_END)
        .step_by(SECTOR)
        .filter(|a| image[*a..*a + SECTOR] != now[*a..*a + SECTOR])
        .collect();
    let low: Vec<_> = diff.iter().filter(|a| **a < WRITE_MIN).collect();
    if !low.is_empty() {
        return Err(fail(format!("the protected head of the flash differs from V15 (sectors {}): not writing; send this output and the backup",low.iter().map(|a|format!("{a:#06x}")).collect::<Vec<_>>().join(" "))));
    }
    Ok(diff)
}
fn u32_value(v: &Value) -> Result<u32> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| Error::value("expected u32"))
}
fn usize_value(v: &Value, max: usize) -> Result<usize> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .filter(|n| *n <= max)
        .ok_or_else(|| Error::value("value outside supported bound"))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub struct Device<'a, A> {
    pub io: &'a mut A,
    pub bulk: bool,
}
impl<A: Adapter> Device<'_, A> {
    fn action(&mut self, kind: &str, args: Value, handled: &[&str]) -> Result<Value> {
        self.io
            .action(json!({"kind":kind,"args":args,"handled_errors":handled}))
    }
    pub fn bot(
        &mut self,
        cdb: &[u8],
        data: Option<&[u8]>,
        n_in: usize,
        timeout: u32,
        handled: &[&str],
    ) -> Result<Vec<u8>> {
        if cdb.len() > 16 || data.is_some_and(|d| d.len() > 1_100_000) || n_in > 1_100_000 {
            return Err(fail("bulk command exceeds supported bounds"));
        }
        self.action("bulk_begin", json!([]), handled)?;
        let result = (|| {
            let tag = u32_value(&self.action("tag", json!([]), handled)?)?;
            self.action("tag_set", json!([tag.wrapping_add(1)]), handled)?;
            let len = data.map_or(n_in, |d| d.len()) as u32;
            let mut cbw = b"USBC".to_vec();
            cbw.extend(tag.to_le_bytes());
            cbw.extend(len.to_le_bytes());
            cbw.extend([if n_in != 0 { 128 } else { 0 }, 0, cdb.len() as u8]);
            cbw.extend(cdb);
            cbw.resize(31, 0);
            self.action("bulk_write", json!([cbw, timeout]), handled)?;
            let mut got = Vec::new();
            if let Some(data) = data.filter(|d| !d.is_empty()) {
                self.action("bulk_write", json!([data, timeout]), handled)?;
            } else if n_in != 0 {
                got = bytes(&self.action("bulk_read", json!([n_in, timeout]), handled)?)?;
            }
            let csw = bytes(&self.action("bulk_read", json!([13, timeout]), handled)?)?;
            if csw.len() != 13 {
                return Err(structure("unpack requires a buffer of 13 bytes"));
            }
            if csw[..4] != *b"USBS" || u32::from_le_bytes(csw[4..8].try_into().unwrap()) != tag {
                return Err(fail(format!("bad status block ({})", hex(&csw))));
            }
            if csw[12] != 0 {
                return Err(fail(format!("command failed (status {})", csw[12])));
            }
            Ok(got)
        })();
        let release = self.action("bulk_end", json!([]), handled);
        match result {
            Err(e) => Err(e),
            Ok(v) => {
                release?;
                Ok(v)
            }
        }
    }
    pub fn inquiry(&mut self) -> Result<(String, String)> {
        if !self.bulk {
            let v = self.action("dev_inquiry", json!([]), &[])?;
            return Ok((text(&v[0])?.into(), text(&v[1])?.into()));
        }
        let b = self.bot(&[18, 0, 0, 0, 36, 0], None, 36, 5000, &[])?;
        let field = |a, end| {
            String::from_utf8_lossy(bounded(&b, a, end))
                .trim()
                .to_string()
        };
        Ok((field(8, 16), field(16, 32)))
    }
    pub fn cmd(&mut self, cmd: u16, args: &[u8], handled: &[&str]) -> Result<Vec<u8>> {
        if !self.bulk {
            return bytes(&self.action("dev_cmd", json!([cmd, args]), handled)?);
        }
        let r = self.bot(&cdb(cmd, args)?, None, 16, 5000, handled)?;
        if r.len() < 2 {
            return Err(structure("unpack requires a buffer of 2 bytes"));
        }
        if u16::from_be_bytes([r[0], r[1]]) != cmd {
            return Err(fail(format!(
                "command {cmd:04X} answered as {}",
                hex(&r[..2])
            )));
        }
        Ok(r[2..].to_vec())
    }
    pub fn write_block(
        &mut self,
        cmd: u16,
        addr: u32,
        data: &[u8],
        handled: &[&str],
    ) -> Result<()> {
        if data.len() > IO {
            return Err(fail("write block exceeds512 bytes"));
        }
        let mut args = addr.to_be_bytes().to_vec();
        args.extend((data.len() as u16).to_be_bytes());
        args.push(0);
        args.extend(package::crc16(data, 0).to_le_bytes());
        self.bot(&cdb(cmd, &args)?, Some(data), 0, 5000, handled)?;
        Ok(())
    }
    pub fn start_loader(&mut self, blob: &[u8]) -> Result<()> {
        if !self.bulk {
            self.action("dev_start_loader", json!([blob]), &[])?;
            return Ok(());
        }
        for (i, b) in blob.chunks(IO).enumerate() {
            self.write_block(WRITE_MEMORY, LOADER_ADDR + (i * IO) as u32, b, &[])?;
        }
        let mut args = LOADER_ADDR.to_be_bytes().to_vec();
        args.extend(1u16.to_be_bytes());
        self.cmd(JUMP, &args, &[])?;
        Ok(())
    }
    pub fn chip(&mut self) -> Result<(u16, u8, u32)> {
        if !self.bulk {
            let v = self.action("dev_chip", json!([]), &[])?;
            return Ok((
                u16::try_from(u32_value(&v[0])?).map_err(|_| Error::value("invalid chip key"))?,
                u8::try_from(u32_value(&v[1])?).map_err(|_| Error::value("invalid flash type"))?,
                u32_value(&v[2])?,
            ));
        }
        let p = self.cmd(READ_KEY, &0x00ac6900u32.to_be_bytes(), &[])?;
        if p.len() < 6 {
            return Err(Error {
                kind: "IndexError".into(),
                code: String::new(),
                message: "index out of range".into(),
            });
        }
        let mut k = [p[5], p[4]];
        tool_wire::crc_cipher(&mut k);
        let p = self.cmd(ONLINE, &[], &[])?;
        if p.len() < 6 {
            return Err(structure(format!("unpack_from requires a buffer of at least 6 bytes for unpacking 4 bytes at offset 2 (actual buffer size is {})",p.len())));
        }
        Ok((
            u16::from_le_bytes(k),
            p[0],
            u32::from_le_bytes(p[2..6].try_into().unwrap()),
        ))
    }
    pub fn read(&mut self, addr: u32, n: usize, handled: &[&str]) -> Result<Vec<u8>> {
        if n > FLASH_SIZE || u64::from(addr) + n as u64 > u64::from(u32::MAX) + 1 {
            return Err(fail("flash read exceeds supported bounds"));
        }
        if !self.bulk {
            return bytes(&self.action("dev_read", json!([addr, n]), handled)?);
        }
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let k = IO.min(n - out.len());
            let mut args = (addr + out.len() as u32).to_be_bytes().to_vec();
            args.extend((k as u16).to_be_bytes());
            let b = self.bot(&cdb(READ_FLASH, &args)?, None, k, 5000, handled)?;
            if b.is_empty() || b.len() > k {
                return Err(fail("short or oversized flash read"));
            }
            out.extend(b);
        }
        Ok(out)
    }
    pub fn write_sector(&mut self, addr: usize, data: &[u8], handled: &[&str]) -> Result<()> {
        if !(WRITE_MIN..APP_END).contains(&addr)
            || !addr.is_multiple_of(SECTOR)
            || data.len() != SECTOR
        {
            return Err(fail(format!("refusing to write {addr:#x}")));
        }
        if !self.bulk {
            self.action("dev_write_sector", json!([addr, data]), handled)?;
            return Ok(());
        }
        self.cmd(ERASE, &(addr as u32).to_be_bytes(), handled)?;
        for (i, b) in data.chunks(IO).enumerate() {
            self.write_block(WRITE_FLASH, (addr + i * IO) as u32, b, handled)?;
        }
        if self.read(addr as u32, SECTOR, handled)? != data {
            return Err(fail(format!("sector {addr:#07x} did not verify")));
        }
        Ok(())
    }
    pub fn run_app(&mut self) -> Result<()> {
        if !self.bulk {
            self.action("dev_run_app", json!([]), &[])?;
            return Ok(());
        }
        let _ = self.cmd(RUN_APP, &1u32.to_be_bytes(), &["*"]);
        Ok(())
    }
}
fn bounded(data: &[u8], a: usize, b: usize) -> &[u8] {
    &data[a.min(data.len())..b.min(data.len())]
}
fn say(io: &mut impl Adapter, s: String, end: Option<&str>) -> Result<()> {
    io.action(json!({"kind":"say","args":[s],"end":end}))?;
    Ok(())
}
fn clock(io: &mut impl Adapter) -> Result<f64> {
    number(&io.action(json!({"kind":"clock","args":[]}))?)
}
/// Recoverable exceptions are classified by the adapter's Python MRO while
/// their policy stays scoped here. No handled failure leaks into a later retry.
struct Handled<'a, A> {
    io: &'a mut A,
    classes: &'static [&'static str],
}
impl<A: Adapter> Adapter for Handled<'_, A> {
    fn action(&mut self, mut action: Value) -> Result<Value> {
        let mut classes = action["handled_errors"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for class in self.classes {
            if !classes.iter().any(|c| c == class) {
                classes.push(json!(class))
            }
        }
        action["handled_errors"] = json!(classes);
        self.io.action(action)
    }
}
fn exception(e: &Error) -> bool {
    ![
        "BaseException",
        "KeyboardInterrupt",
        "SystemExit",
        "GeneratorExit",
    ]
    .contains(&e.kind.as_str())
}
pub fn reset(io: &mut impl Adapter) -> Result<()> {
    for kind in ["bulk_reset", "bulk_clear_in", "bulk_clear_out"] {
        if let Err(e) = io.action(json!({"kind":kind,"args":[],"handled_errors":["USBError"]})) {
            if e.kind != "USBError" {
                return Err(e);
            }
        }
    }
    Ok(())
}
pub fn open(io: &mut impl Adapter, bulk: bool, timeout: f64) -> Result<()> {
    say(
        io,
        "waiting for the FM-1 in update mode (switch it on, or wait for it to cycle) ...".into(),
        None,
    )?;
    io.action(json!({"kind":"usb_backend","args":[]}))?;
    let t0 = clock(io)?;
    while clock(io)? - t0 < timeout {
        if io.action(json!({"kind":"usb_find","args":[0x4c4a,0x8057]}))? == true {
            let elapsed = clock(io)? - t0;
            say(io, format!("found it after {elapsed:.1} s"), None)?;
            let result = (|| {
                let mut handled = Handled {
                    io: &mut *io,
                    classes: &["*"],
                };
                handled.action(json!({"kind":"usb_detach","args":[]}))?;
                handled.action(json!({"kind":"usb_open","args":[]}))?;
                if let Err(e) = (Device {
                    io: &mut handled,
                    bulk,
                })
                .inquiry()
                {
                    if !exception(&e) {
                        return Err(e);
                    }
                    reset(&mut handled)?;
                }
                Ok(())
            })();
            match result {
                Ok(()) => return Ok(()),
                Err(e) if exception(&e) => {
                    say(
                        io,
                        format!(
                            "  could not open it ({}); waiting for the next cycle",
                            e.message
                        ),
                        None,
                    )?;
                    io.action(json!({"kind":"sleep","args":[1.0]}))?;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        io.action(json!({"kind":"sleep","args":[0.02]}))?;
    }
    Err(fail("the FM-1 did not appear in update mode"))
}
#[derive(Clone, Copy)]
pub struct RecoveryOptions<'a> {
    pub write: bool,
    pub backup_dir: &'a str,
    pub confirm: bool,
    pub tries: i64,
    pub wait: f64,
}
pub fn recover(
    io: &mut impl Adapter,
    bulk: bool,
    image: &[u8],
    blob: &[u8],
    options: RecoveryOptions<'_>,
) -> Result<u8> {
    let RecoveryOptions {
        write,
        backup_dir: dir,
        confirm,
        tries,
        wait,
    } = options;
    if tries > 1000 {
        return Err(Error::value("recovery tries exceeds1000"));
    }
    for attempt in 1..=tries {
        open(io, bulk, wait)?;
        let result = restore(
            &mut Handled {
                io: &mut *io,
                classes: &["Fail", "USBError"],
            },
            bulk,
            image,
            blob,
            write,
            dir,
            confirm,
        );
        let dispose = |io: &mut dyn Adapter| {
            io.action(json!({"kind":"usb_dispose","args":[],"handled_errors":["*"]}))
        };
        match result {
            Ok(_) => {
                let _ = dispose(io);
                return Ok(0);
            }
            Err(e) if e.kind == "Fail" || e.kind == "USBError" => {
                let printed = say(io, format!("\n  attempt {attempt}: {}", e.message), None);
                let _ = dispose(io);
                printed?;
                if (e.kind == "Fail" && e.message.contains("not writing"))
                    || e.message.contains("not the FM-1")
                {
                    return Ok(1);
                }
            }
            Err(e) => {
                let _ = dispose(io);
                return Err(e);
            }
        }
    }
    Err(fail("gave up; send this whole output"))
}
pub fn restore(
    io: &mut impl Adapter,
    bulk: bool,
    image: &[u8],
    blob: &[u8],
    mut write: bool,
    backup_dir: &str,
    confirm: bool,
) -> Result<bool> {
    if image.len() != APP_END {
        return Err(fail("unexpected flash entry size"));
    }
    let (vendor, product) = Device { io, bulk }.inquiry()?;
    say(io, format!("  device: {vendor} {product}"), None)?;
    if vendor != "WL82" {
        return Err(fail(format!(
            "not a WL82 UBOOT ({})",
            crate::installer::repr(&vendor)
        )));
    }
    Device { io, bulk }.start_loader(blob)?;
    let (key, typ, fid) = Device { io, bulk }.chip()?;
    say(
        io,
        format!("  chip key {key:04X}, flash type {typ}, id {fid:06X}"),
        None,
    )?;
    if (key, typ) != (0x980f, 3) || ![0x856014, 0xb36014].contains(&fid) {
        return Err(fail("this is not the FM-1's chip and flash (expected key 980F, type 3, id 856014 or B36014); nothing written"));
    }
    say(io, "  reading the whole flash ...".into(), None)?;
    let t0 = clock(io)?;
    let now = Device { io, bulk }.read(0, FLASH_SIZE, &[])?;
    if now.len() != FLASH_SIZE {
        return Err(fail("unexpected flash snapshot size"));
    }
    let stamp = io.action(json!({"kind":"timestamp","args":[]}))?;
    let name = format!("fm1-backup-{}.bin", text(&stamp)?);
    let path = std::path::Path::new(backup_dir)
        .join(name)
        .to_string_lossy()
        .into_owned();
    io.action(json!({"kind":"backup","args":[path,now]}))?;
    let elapsed = clock(io)? - t0;
    say(io, format!("  backup saved: {path} ({elapsed:.1} s)"), None)?;
    let diff = plan(image, &now)?;
    say(
        io,
        format!(
            "  {} of {} firmware sectors differ from V15",
            diff.len(),
            APP_END / SECTOR
        ),
        None,
    )?;
    if !write && confirm {
        write = io.action(json!({"kind":"confirm_keep_alive","args":[]}))? == true;
    }
    if !write {
        say(
            io,
            "  check passed. Nothing written: run again with --write to put V15 back.".into(),
            None,
        )?;
        return Ok(false);
    }
    for (i, addr) in diff.iter().rev().enumerate() {
        for attempt in 0..3 {
            match (Device { io: &mut *io, bulk }).write_sector(
                *addr,
                &image[*addr..*addr + SECTOR],
                &["Fail"],
            ) {
                Ok(()) => break,
                Err(e) if e.kind == "Fail" => {
                    if attempt == 2 {
                        return Err(fail(format!(
                            "{}; the FM-1 is still in update mode: run the script again",
                            e.message
                        )));
                    }
                }
                Err(e) => return Err(e),
            }
        }
        say(
            io,
            format!("\r  writing {}/{}", i + 1, diff.len()),
            Some(""),
        )?;
    }
    say(io, String::new(), None)?;
    say(io, "  final read ...".into(), None)?;
    let mut expect = image.to_vec();
    expect.extend(&now[APP_END..]);
    let final_image = Device { io, bulk }.read(0, FLASH_SIZE, &[])?;
    if final_image != expect {
        let bad = (0..FLASH_SIZE)
            .step_by(SECTOR)
            .filter(|a| bounded(&final_image, *a, *a + SECTOR) != &expect[*a..*a + SECTOR])
            .map(|a| format!("{a:#07x}"))
            .collect::<Vec<_>>()
            .join(" ");
        return Err(fail(format!(
            "final read differs in sectors {bad}; run the script again with --write"
        )));
    }
    say(
        io,
        "  flash now holds V15. Restarting the FM-1 ...".into(),
        None,
    )?;
    Device { io, bulk }.run_app()?;
    Ok(true)
}
pub fn request(method: &str, v: &Value, io: &mut impl Adapter) -> Result<Value> {
    let a = &v["args"];
    let bulk = v["bulk"] == true;
    Ok(match method {
        "config" => config(),
        "sha256" => json!(sha256(&bytes(&a[0])?)),
        "stock" => json!(stock(&bytes(&a[0])?, text(&a[1])?)?),
        "loader" => {
            loader(&bytes(&a[0])?)?;
            Value::Null
        }
        "uid" => {
            if a[0] != 0 {
                return Err(fail("run it with sudo: macOS has to give the device up"));
            }
            Value::Null
        }
        "reset" => {
            reset(io)?;
            Value::Null
        }
        "open" => {
            open(io, bulk, number(&a[0])?)?;
            Value::Null
        }
        "recover" => json!(recover(
            io,
            bulk,
            &bytes(&a[0])?,
            &bytes(&a[1])?,
            RecoveryOptions {
                write: a[2] == true,
                backup_dir: text(&a[3])?,
                confirm: a[4] == true,
                tries: a[5]
                    .as_i64()
                    .ok_or_else(|| Error::value("expected bounded tries"))?,
                wait: number(&a[6])?
            }
        )?),
        "plan" => json!(plan(&bytes(&a[0])?, &bytes(&a[1])?)?),
        "cdb" => json!(cdb(
            u16::try_from(u32_value(&a[0])?).map_err(|_| Error::value("expected u16 command"))?,
            &bytes(&a[1])?
        )?),
        "restore" => json!(restore(
            io,
            bulk,
            &bytes(&a[0])?,
            &bytes(&a[1])?,
            a[2] == true,
            text(&a[3])?,
            a[4] == true
        )?),
        "bot" => json!(Device { io, bulk: true }.bot(
            &bytes(&a[0])?,
            if a[1].is_null() {
                None
            } else {
                Some(bytes(&a[1])?)
            }
            .as_deref(),
            usize_value(&a[2], 1_100_000)?,
            u32_value(&a[3])?,
            &[]
        )?),
        "inquiry" => {
            let (v, p) = Device { io, bulk }.inquiry()?;
            json!([v, p])
        }
        "cmd" => json!(Device { io, bulk }.cmd(
            u16::try_from(u32_value(&a[0])?).map_err(|_| Error::value("expected u16 command"))?,
            &bytes(&a[1])?,
            &[]
        )?),
        "start_loader" => {
            Device { io, bulk }.start_loader(&bytes(&a[0])?)?;
            Value::Null
        }
        "chip" => {
            let (k, t, f) = Device { io, bulk }.chip()?;
            json!([k, t, f])
        }
        "read" => json!(Device { io, bulk }.read(
            u32_value(&a[0])?,
            usize_value(&a[1], FLASH_SIZE)?,
            &[]
        )?),
        "write_sector" => {
            if let Some(n) = a[0].as_i64().filter(|n| *n < 0) {
                return Err(fail(format!("refusing to write -{:#x}", n.unsigned_abs())));
            }
            Device { io, bulk }.write_sector(
                usize_value(&a[0], usize::MAX)?,
                &bytes(&a[1])?,
                &[],
            )?;
            Value::Null
        }
        "write_block" => {
            Device { io, bulk }.write_block(
                u16::try_from(u32_value(&a[0])?)
                    .map_err(|_| Error::value("expected u16 command"))?,
                u32_value(&a[1])?,
                &bytes(&a[2])?,
                &[],
            )?;
            Value::Null
        }
        "run_app" => {
            Device { io, bulk }.run_app()?;
            Value::Null
        }
        "keep_alive" => {
            loop {
                if io.action(json!({"kind":"stop_wait","args":[1.0]}))? == true {
                    break;
                }
                if let Err(e) = (Device { io: &mut *io, bulk }).cmd(ONLINE, &[], &["*"]) {
                    return Ok(json!({"lost":e.message}));
                }
            }
            json!({"lost":null})
        }
        _ => return Err(Error::value("unknown rescue method")),
    })
}
