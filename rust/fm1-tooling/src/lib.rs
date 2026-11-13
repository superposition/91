// SPDX-License-Identifier: GPL-3.0-only
//! Offline installer and diagnostic policy. Actual ports and clocks are supplied
//! by an explicit synchronous adapter; this crate never opens a device.
#![forbid(unsafe_code)]
pub mod assets;
pub mod build;
pub mod build_guards;
pub mod debugger;
pub mod installer;
pub mod lz4;
pub mod mmio;
pub mod rescue;
pub mod summary;
pub mod telemetry;
use serde_json::{json, Value};
use std::io::{Read, Write};
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone)]
pub struct Error {
    pub kind: String,
    pub code: String,
    pub message: String,
}
impl Error {
    pub fn value(message: impl Into<String>) -> Self {
        Self {
            kind: "ValueError".into(),
            code: String::new(),
            message: message.into(),
        }
    }
    pub fn install(code: &str, message: impl Into<String>) -> Self {
        Self {
            kind: "InstallError".into(),
            code: code.into(),
            message: message.into(),
        }
    }
    fn observed(v: &Value) -> Self {
        Self {
            kind: v["kind"].as_str().unwrap_or("RuntimeError").into(),
            code: v["code"].as_str().unwrap_or("").into(),
            message: v["message"].as_str().unwrap_or("adapter failed").into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self {
            kind: "OSError".into(),
            code: String::new(),
            message: e.to_string(),
        }
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::value(e.to_string())
    }
}
pub trait Adapter {
    fn action(&mut self, action: Value) -> Result<Value>;
}
pub fn bytes(v: &Value) -> Result<Vec<u8>> {
    let a = v
        .as_array()
        .ok_or_else(|| Error::value("expected byte array"))?;
    if a.len() > 1_100_000 {
        return Err(Error::value("byte array exceeds 1100000 bytes"));
    }
    a.iter()
        .map(|n| {
            n.as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| Error::value("expected byte 0..255"))
        })
        .collect()
}
pub fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| Error::value("expected string"))
}
pub fn number(v: &Value) -> Result<f64> {
    v.as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| Error::value("expected finite number"))
}
pub fn request(v: &Value, io: &mut impl Adapter) -> Result<Value> {
    let method = text(&v["method"])?;
    if let Some(method) = method.strip_prefix("build.") {
        build::request(method, v)
    } else if let Some(method) = method.strip_prefix("install.") {
        installer::request(method, v, io)
    } else if let Some(method) = method.strip_prefix("rescue.") {
        rescue::request(method, v, io)
    } else if let Some(method) = method.strip_prefix("debug.") {
        debugger::request(method, v, io)
    } else if let Some(method) = method.strip_prefix("asset.") {
        assets::request(method, v, io)
    } else {
        telemetry::request(method, v)
    }
}
const CAP: usize = 32 * 1024 * 1024;
fn read_frame(input: &mut impl Read) -> Result<Option<Value>> {
    let mut n = [0u8; 4];
    if input.read(&mut n[..1])? == 0 {
        return Ok(None);
    }
    input.read_exact(&mut n[1..])?;
    let n = u32::from_le_bytes(n) as usize;
    if n > CAP {
        return Err(Error::value("tool request exceeds 32 MiB"));
    }
    let mut b = vec![0; n];
    input.read_exact(&mut b)?;
    Ok(Some(serde_json::from_slice(&b)?))
}
fn write_frame(output: &mut impl Write, v: &Value) -> Result<()> {
    let b = serde_json::to_vec(v)?;
    if b.len() > CAP {
        return Err(Error::value("tool response exceeds 32 MiB"));
    }
    output.write_all(&(b.len() as u32).to_le_bytes())?;
    output.write_all(&b)?;
    output.flush()?;
    Ok(())
}
struct Pipe<'a, R, W> {
    input: &'a mut R,
    output: &'a mut W,
}
impl<R: Read, W: Write> Adapter for Pipe<'_, R, W> {
    fn action(&mut self, action: Value) -> Result<Value> {
        write_frame(self.output, &json!({"action":action}))?;
        let response = read_frame(self.input)?
            .ok_or_else(|| Error::value("adapter closed during transaction"))?;
        if response["ok"] == true {
            Ok(response["value"].clone())
        } else {
            Err(Error::observed(&response))
        }
    }
}
/// Serialized sessions retain only their stack and one package image. On return
/// all Rust session state is dropped, including failure paths; no link is owned
/// here and no asynchronous callbacks can re-enter a mutable transaction.
pub fn stream() -> Result<()> {
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    while let Some(v) = read_frame(&mut input)? {
        let result = request(
            &v,
            &mut Pipe {
                input: &mut input,
                output: &mut output,
            },
        );
        let v = match result {
            Ok(v) => json!({"ok":true,"value":v}),
            Err(e) => json!({"ok":false,"kind":e.kind,"code":e.code,"message":e.message}),
        };
        write_frame(&mut output, &v)?;
    }
    Ok(())
}
pub fn cli(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("loader-wrap") if args.len()==3 => {
            let image=read_file(&args[1],1_100_000)?;
            std::fs::write(&args[2],build::loader_wrap(&image)?)?;
        }
        Some("lz4-compress"|"lz4-decompress") if args.len()==3 => {
            let image=read_file(&args[1],1_110_000)?;
            let output=if args[0]=="lz4-compress" {lz4::compress(&image)?}else{lz4::decompress(&image,&[])?};
            std::fs::write(&args[2],output)?;
        }
        Some(kind @ ("manual-body"|"quick-html")) if args.len()==3=>{
            let source=String::from_utf8(read_file(&args[1],16*1024*1024)?).map_err(|_|Error::value("manual must be UTF-8"))?;
            let source=source.replace("\r\n","\n").replace('\r',"\n");
            std::fs::write(&args[2],assets::manual::native(&source,kind=="quick-html")?)?;
        }
        Some("pdf-count") if args.len()==2||(args.len()==3&&args[2]=="--full")=>{
            let data=read_file(&args[1],12*1024*1024)?;
            println!("{}",json!({"pages":assets::manual::pdf::count(&data,args.len()==3),"bytes":data.len()}));
        }
        Some("dy-build") if args.len()==4||args.len()==5 => {
            let mut argv=vec![args[1].clone(),args[2].clone()];
            if let Some(rom)=args.get(4){argv.push("--rom".into());argv.push(rom.clone());}
            assets::dy::generate(&mut assets::dy::Native::default(),&json!(argv),&args[3],"dy-build MANIFEST BANKS OUTROOT [ROMDIR]")?;
        }
        Some("logo-rgba") if args.len()==5 => {
            let source=String::from_utf8(read_file(&args[1],16*1024*1024)?).map_err(|_|Error::value("sprite source must be UTF-8"))?;
            let rows=assets::logo::sprite(&source)?;
            let image=assets::logo::render(&json!(rows),cli_integer(&args[2])?,Some(cli_integer(&args[3])?),&assets::logo::palette())?;
            std::fs::write(&args[4],&image.rgba)?;
            println!("{}",json!({"width":image.width,"height":image.height,"rgba_bytes":image.rgba.len()}));
        }
        Some(kind @ ("sine" | "patterns")) if args.len() == 1 => {
            let report = if kind == "sine" { assets::sine_header() }
                else { assets::pattern_header(&assets::pattern_data())? };
            #[cfg(windows)]
            let report = report.replace('\n', "\r\n");
            std::io::stdout().write_all(report.as_bytes())?;
        }
        Some("rescue-plan") if args.len()==3=>{
            let image=rescue::stock(&read_file(&args[1],1_100_000)?,&args[1])?;
            let now=read_file(&args[2],rescue::FLASH_SIZE)?;
            let changed=rescue::plan(&image,&now)?;
            println!("{}",json!({"changed_sectors":changed,"write_order":changed.iter().rev().copied().collect::<Vec<_>>(),"protected_head_bytes":rescue::WRITE_MIN,"preserved_upper_bytes":rescue::FLASH_SIZE-rescue::APP_END,"writes_performed":0}));
        }
        Some("debug-plan") if args.len()==3||args.len()==4=>{
            let a=cli_integer(&args[2])?;
            let b=if let Some(value)=args.get(3){cli_integer(value)?}else if args[1]=="peek"{1}else if args[1]=="clock"{0}else{return Err(Error::value("poke needs address and value"))};
            println!("{}",debugger::plan(&args[1],a,b)?);
        }
        Some("package-info") if args.len() == 2 || (args.len() == 3 && args[2]=="--force") => {
            let b = read_file(&args[1],1_100_000)?;
            let (product, image) =
                installer::package(&b, args.get(2).is_some_and(|s| s == "--force"), &args[1])?;
            println!("{}", json!({"product":product,"logical_bytes":image.len()}));
        }
        Some("decode") if args.len() == 3 => {
            let kind = args[1]
                .parse::<u8>()
                .map_err(|_| Error::value("frame kind must be a byte"))?;
            let b = read_file(&args[2],1_100_000)?;
            println!("{}", telemetry::decode(kind, &b)?);
        }
        Some("summary") if args.len() == 2 => {
            let rows=summary::csv(&read_file(&args[1],32*1024*1024)?)?;
            for section in ["peaks","halves","fixed","diag","main"] {
                let report=summary::report(section,&rows)?;
                #[cfg(windows)]
                let report=report.replace('\n',"\r\n");
                std::io::stdout().write_all(report.as_bytes())?;
            }
        }
        _ => {
            return Err(Error::value(
                "usage: fm1-rs tools package-info PACKAGE [--force] | decode KIND RAW-SYSEX-DATA | summary TELEMETRY.csv | rescue-plan V15-PACKAGE SNAPSHOT | debug-plan peek|poke|clock A [B] | sine | patterns | lz4-compress IN OUT | lz4-decompress IN OUT | loader-wrap IN OUT | dy-build MANIFEST BANKS OUTROOT [ROMDIR] | logo-rgba UI SCALE SIZE OUTPUT | manual-body INPUT OUTPUT | quick-html INPUT OUTPUT | pdf-count FILE [--full]",
            ))
        }
    }
    Ok(())
}
fn cli_integer(text: &str) -> Result<i64> {
    let text = text.trim();
    let (negative, text) = if let Some(s) = text.strip_prefix('-') {
        (true, s)
    } else {
        (false, text.strip_prefix('+').unwrap_or(text))
    };
    let (radix, text) = if let Some(t) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))
    {
        (16, t)
    } else if let Some(t) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
        (8, t)
    } else if let Some(t) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        (2, t)
    } else {
        (10, text)
    };
    let n =
        i64::from_str_radix(text, radix).map_err(|_| Error::value("expected bounded integer"))?;
    Ok(if negative { -n } else { n })
}
fn read_file(path: &str, max: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err(Error::value(format!("file exceeds {max} bytes")));
    }
    Ok(bytes)
}
