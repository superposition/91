// SPDX-License-Identifier: GPL-3.0-only
// Policy derived from the Felucca installer by Leo Kuroshita and contributors.
use crate::{bytes, number, text, Adapter, Error, Result};
use fm1_formats::{device_wire, package::Fwsc};
use serde_json::{json, Value};
pub const HS_QUERY: [u8; 10] = [240, 0, 50, 69, 0, 0, 0, 64, 127, 247];
pub const UPGRADE: [u8; 6] = [240, 34, 36, 53, 127, 247];
pub const FINISH_CHECK: u32 = 0xe0000000;
pub const FINISH_WRITE: u32 = 0xf0000000;
const LOADER_MARK: &[u8] = b"FELUCCA-LOADER-1";
fn codec(op: u32, args: &[u32], data: &[u8]) -> Result<Vec<u8>> {
    let mut out = vec![0; 640];
    let n = device_wire::dispatch(op, args, data, &mut out)
        .map_err(|_| Error::value("invalid installer wire packet"))?;
    out.truncate(n);
    Ok(out)
}
fn latin1(b: &[u8]) -> String {
    b.iter().map(|b| char::from(*b)).collect()
}
pub fn identity(pkt: &[u8]) -> Option<Value> {
    if pkt.len() < 2 || pkt.first() != Some(&240) || pkt.last() != Some(&247) {
        return None;
    }
    let b = codec(94, &[1], pkt).ok()?;
    let t = latin1(b.strip_suffix(&[0]).map_or(&b, |_| {
        let mut n = b.len();
        while n > 0 && b[n - 1] == 0 {
            n -= 1
        }
        &b[..n]
    }));
    let (model, num) = t.split_once('_')?;
    if model.is_empty() || num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // Keep the bounded decimal text if it exceeds JSON's u64 range. The Python
    // identity view converts it to its original arbitrary-precision integer.
    let version = num.parse::<u64>().map_or_else(|_| json!(num), |n| json!(n));
    Some(
        json!({"text":t,"model":model,"version":version,"loader":model.to_lowercase().starts_with("ota-")}),
    )
}
pub fn model(t: &str) -> &str {
    let m = t.split('_').next().unwrap_or("");
    m.strip_prefix("ota-").unwrap_or(m)
}
fn base_port(s: &str) -> &str {
    let end = s.trim_end_matches(|c: char| c.is_ascii_digit());
    if end.len() != s.len() && end.chars().last().is_some_and(char::is_whitespace) {
        end.trim_end()
    } else {
        s
    }
}
pub fn pair_output(name: &str, outs: &[Value]) -> Value {
    if outs.iter().any(|v| v == name) {
        return json!(name);
    }
    let same: Vec<_> = outs
        .iter()
        .filter(|v| v.as_str().is_some_and(|s| base_port(s) == base_port(name)))
        .collect();
    if same.len() == 1 {
        same[0].clone()
    } else if outs.len() == 1 {
        outs[0].clone()
    } else {
        Value::Null
    }
}
pub fn candidates(names: &[Value], port: Option<&str>, only: bool) -> Vec<Value> {
    let allowed = |n: &Value| {
        n.as_str().is_some_and(|n| {
            let n = n.to_lowercase();
            ["fm-1", "felucca", "ota", "composite", "sinco", "usb-midi"]
                .iter()
                .any(|k| n.contains(k))
        })
    };
    if let Some(port) = port.filter(|p| !p.is_empty()) {
        let port = port.to_lowercase();
        let mut mine: Vec<_> = names
            .iter()
            .filter(|n| n.as_str().is_some_and(|n| n.to_lowercase().contains(&port)))
            .cloned()
            .collect();
        if !only {
            for n in names {
                if !mine.contains(n) && allowed(n) {
                    mine.push(n.clone())
                }
            }
        }
        mine
    } else {
        names.iter().filter(|n| allowed(n)).cloned().collect()
    }
}
fn repr(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut o = q.to_string();
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if c == q => {
                o.push('\\');
                o.push(c)
            }
            c => o.push(c),
        }
    }
    o.push(q);
    o
}
pub fn package(raw: &[u8], force: bool, path: &str) -> Result<(String, Vec<u8>)> {
    let bad = || Error::install("badpkg", "not an FM-1 package (too short or damaged)");
    let p = Fwsc::parse(raw).map_err(|_| bad())?;
    let mut product = [0; 20];
    let n = p.product(&mut product).map_err(|_| bad())?;
    let product = latin1(&product[..n]);
    let valid = product.split_once('_').is_some_and(|(m, n)| {
        !m.is_empty() && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())
    });
    if !valid {
        return Err(Error::install(
            "badpkg",
            format!("{path}: not an FM-1 package (identity {})", repr(&product)),
        ));
    }
    if !force && !raw.windows(LOADER_MARK.len()).any(|b| b == LOADER_MARK) {
        return Err(Error::install("badpkg",format!("{path}: no Felucca update loader in this package; only Felucca's own packages are installed (--force overrides)")));
    }
    let mut image = vec![0; p.logical_len()];
    p.copy_logical(0, &mut image).map_err(|_| bad())?;
    Ok((product, image))
}
struct Updater<'a, A> {
    io: &'a mut A,
    port: Option<String>,
    delay: Value,
}
impl<A: Adapter> Updater<'_, A> {
    fn call(&mut self, kind: &str, args: Value) -> Result<Value> {
        self.io.action(json!({"kind":kind,"args":args}))
    }
    fn clock(&mut self) -> Result<f64> {
        number(&self.call("clock", json!([]))?)
    }
    fn delay(&self, name: &str) -> Result<f64> {
        number(&self.delay[name])
    }
    fn sleep(&mut self, name: &str) -> Result<()> {
        let d = self.delay(name)?;
        self.call("sleep", json!([d]))?;
        Ok(())
    }
    fn send(&mut self, link: &Value, packet: &[u8]) -> Result<bool> {
        Ok(self.call("send", json!([link, packet]))? == true)
    }
    fn close(&mut self, link: &Value) -> Result<()> {
        self.call("close", json!([link]))?;
        Ok(())
    }
    fn step(&mut self, name: &str, value: Value) -> Result<()> {
        self.call("step", json!([name, value]))?;
        Ok(())
    }
    fn ask(&mut self, link: &Value, query: &[u8], secs: f64) -> Result<Value> {
        if !self.send(link, query)? {
            return Ok(json!(false));
        }
        let end = self.clock()? + secs;
        loop {
            let left = end - self.clock()?;
            if left <= 0.0 {
                break;
            }
            let p = self.call("read", json!([link, left]))?;
            if p.is_null() {
                break;
            }
            if let Some(id) = identity(&bytes(&p)?) {
                return Ok(id);
            }
        }
        Ok(Value::Null)
    }
    fn handshake(&mut self, link: &Value, tries: u64) -> Result<Value> {
        self.call("drain", json!([link]))?;
        for _ in 0..tries {
            let id = self.ask(link, &HS_QUERY, self.delay("hs")?)?;
            if id == false {
                return Ok(Value::Null);
            }
            if id.is_null() {
                continue;
            }
            let t = text(&id["text"])?;
            let num = t.split_once('_').unwrap().1;
            if num.len() != 3 || !num.starts_with('7') {
                return Ok(id);
            }
            let full = self.ask(link, &codec(95, &[1], &[])?, 0.6)?;
            return Ok(if full.is_object() && full["model"] == id["model"] {
                full
            } else {
                id
            });
        }
        Ok(Value::Null)
    }
    fn find(&mut self, only: bool, want: &str) -> Result<Value> {
        let outs = self.call("outputs", json!([]))?;
        let names = self.call("inputs", json!([]))?;
        let names = names
            .as_array()
            .ok_or_else(|| Error::value("invalid input ports"))?;
        let outs = outs
            .as_array()
            .ok_or_else(|| Error::value("invalid output ports"))?;
        for name in candidates(names, self.port.as_deref(), only) {
            let out = pair_output(text(&name)?, outs);
            if out.is_null() || out == "" {
                continue;
            }
            let link = match self.call("open", json!([name, out])) {
                Ok(link) => link,
                Err(e) if e.kind == "KeyboardInterrupt" || e.kind == "SystemExit" => return Err(e),
                Err(_) => continue,
            };
            let id = self.handshake(&link, 2)?;
            let accepted = if id.is_null() {
                false
            } else {
                match want {
                    "loader" => id["loader"] == true,
                    "running" => id["loader"] == false,
                    "callback" => self.call("want", json!([id]))? == true,
                    _ => true,
                }
            };
            if accepted {
                return Ok(json!({"link":link,"id":id,"name":name}));
            }
            self.close(&link)?;
        }
        Ok(Value::Null)
    }
    fn wait(&mut self, want: &str, secs: f64) -> Result<Value> {
        let end = self.clock()? + secs;
        while self.clock()? < end {
            let dev = self.find(false, want)?;
            if !dev.is_null() {
                return Ok(dev);
            }
            self.sleep("retry")?;
        }
        Ok(Value::Null)
    }
    fn serve(
        &mut self,
        link: &Value,
        image: &[u8],
        finish: u32,
        idle: f64,
        progress: &str,
    ) -> Result<(u64, bool, bool)> {
        let (mut served, mut last) = (0, self.clock()?);
        loop {
            let pkt = self.call("read", json!([link, self.delay("poll")?]))?;
            if pkt.is_null() {
                if self.call("lost", json!([link]))? == true {
                    return Ok((served, false, true));
                }
                if self.clock()? - last > idle {
                    return Ok((served, false, false));
                }
                continue;
            }
            let Ok(r) = codec(94, &[0], &bytes(&pkt)?) else {
                continue;
            };
            let word = |n| u32::from_le_bytes(r[n..n + 4].try_into().unwrap());
            let (fl, addr, n) = (word(0), word(4), word(8));
            last = self.clock()?;
            if addr == FINISH_CHECK || addr == FINISH_WRITE {
                self.send(link, &codec(93, &[addr, 0], b"success\0")?)?;
                if addr == finish {
                    return Ok((served, true, false));
                }
                continue;
            }
            if n > 512 || (addr as u64 + n as u64) > image.len() as u64 {
                return Err(Error::install(
                    "badreq",
                    format!("the device asked for {addr:#x}+{n}, outside the package"),
                ));
            }
            self.sleep("reply")?;
            if !self.send(
                link,
                &codec(
                    93,
                    &[addr, fl],
                    &image[addr as usize..addr as usize + n as usize],
                )?,
            )? {
                return Ok((served, false, true));
            }
            served += 1;
            let end = addr as u64 + n as u64;
            if progress == "raw" {
                self.call("progress", json!([served, end]))?;
            } else if progress == "check" {
                self.step("check", json!(served))?;
            } else {
                let percent = (end * 100)
                    .checked_div(image.len() as u64)
                    .ok_or_else(|| Error {
                        kind: "ZeroDivisionError".into(),
                        code: String::new(),
                        message: "integer division or modulo by zero".into(),
                    })?;
                self.step("write", json!(percent.min(99)))?;
            }
        }
    }
    fn check(&mut self, dev: &Value, image: &[u8]) -> Result<Value> {
        self.step("start", dev["id"]["text"].clone())?;
        self.send(&dev["link"], &UPGRADE)?;
        self.sleep("start")?;
        let (n, finished, lost) = self.serve(
            &dev["link"],
            image,
            FINISH_CHECK,
            self.delay("idle_check")?,
            "check",
        )?;
        self.close(&dev["link"])?;
        if !finished {
            return Err(Error::install(
                if lost { "lost" } else { "stopped" },
                format!(
                    "the FM-1 {} after {n} requests; nothing was written",
                    if lost {
                        "was disconnected"
                    } else {
                        "stopped answering"
                    }
                ),
            ));
        }
        self.step("loader", Value::Null)?;
        self.sleep("loader")?;
        let dev = self.wait("loader", self.delay("wait_loader")?)?;
        if dev.is_null() {
            return Err(Error::install("noloader","the update loader did not appear. Replug the USB cable and run the install again: the FM-1 stays in update mode until it is done"));
        }
        Ok(dev)
    }
    fn write(&mut self, dev: &Value, image: &[u8]) -> Result<()> {
        self.step("write", json!(0))?;
        self.send(&dev["link"], &UPGRADE)?;
        self.sleep("start")?;
        let (n, finished, lost) = self.serve(
            &dev["link"],
            image,
            FINISH_WRITE,
            self.delay("idle_write")?,
            "write",
        )?;
        self.close(&dev["link"])?;
        if !finished {
            return Err(Error::install(if lost{"lost"}else{"stopped"},format!("the loader {} after {n} requests. Replug the USB cable and run the install again to finish",if lost{"was disconnected"}else{"stopped answering"})));
        }
        self.step("write", json!(100))?;
        Ok(())
    }
    fn verify(&mut self, product: &str) -> Result<Value> {
        self.step("reboot", Value::Null)?;
        self.sleep("reboot")?;
        let dev = self.wait("running", self.delay("wait_reboot")?)?;
        if dev.is_null() {
            return Err(Error::install(
                "noreturn",
                "the FM-1 did not come back: power-cycle it",
            ));
        }
        self.close(&dev["link"])?;
        if dev["id"]["text"] != product {
            return Err(Error::install(
                "mismatch",
                format!(
                    "written, but the FM-1 reports {} (expected {product})",
                    text(&dev["id"]["text"])?
                ),
            ));
        }
        self.step("done", dev["id"]["text"].clone())?;
        Ok(dev["id"]["text"].clone())
    }
}
pub fn request(method: &str, v: &Value, io: &mut impl Adapter) -> Result<Value> {
    let a = &v["args"];
    match method {
        "identity" => return Ok(identity(&bytes(&a[0])?).unwrap_or(Value::Null)),
        "model" => return Ok(json!(model(text(&a[0])?))),
        "pair" => {
            return Ok(pair_output(
                text(&a[0])?,
                a[1].as_array()
                    .ok_or_else(|| Error::value("expected port list"))?,
            ))
        }
        "candidates" => {
            return Ok(json!(candidates(
                a[0].as_array()
                    .ok_or_else(|| Error::value("expected port list"))?,
                a[1].as_str(),
                a[2] == true
            )))
        }
        "package" => {
            let (p, b) = package(&bytes(&a[0])?, a[1] == true, text(&a[2])?)?;
            return Ok(json!([p, b]));
        }
        _ => {}
    }
    let mut u = Updater {
        io,
        port: v["port"].as_str().map(str::to_owned),
        delay: v["delay"].clone(),
    };
    Ok(match method {
        "ask" => u.ask(&a[0], &bytes(&a[1])?, number(&a[2])?)?,
        "handshake" => u.handshake(
            &a[0],
            a[1].as_u64()
                .filter(|n| *n <= 1000)
                .ok_or_else(|| Error::value("tries must be 0..1000"))?,
        )?,
        "find" => u.find(a[0] == true, text(&a[1])?)?,
        "wait" => u.wait("callback", number(&a[0])?)?,
        "serve" => {
            let image = bytes(&a[1])?;
            let finish = a[2]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| Error::value("invalid finish address"))?;
            let (n, f, l) = u.serve(&a[0], &image, finish, number(&a[3])?, "raw")?;
            json!([n, f, l])
        }
        "check" => u.check(&a[0], &bytes(&a[1])?)?,
        "write" => {
            u.write(&a[0], &bytes(&a[1])?)?;
            Value::Null
        }
        "verify" => u.verify(text(&a[0])?)?,
        "install" => {
            let dev = &a[0];
            let image = bytes(&a[1])?;
            let product = text(&a[2])?;
            if model(text(&dev["id"]["text"])?) != model(product) {
                u.close(&dev["link"])?;
                return Err(Error::install(
                    "model",
                    format!(
                        "the device is {}, the package is for {product}",
                        text(&dev["id"]["text"])?
                    ),
                ));
            }
            let ota = if dev["id"]["loader"] == true {
                dev.clone()
            } else {
                u.check(dev, &image)?
            };
            u.write(&ota, &image)?;
            u.verify(product)?
        }
        _ => return Err(Error::value("unknown installer method")),
    })
}
