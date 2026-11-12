// SPDX-License-Identifier: GPL-3.0-only
mod abi;
use abi::{platform, Runner};
use fm1_sim::{
    clock::Clock,
    platform::STORE_MAX,
    scenario::{self, Comparison, BLACK, BUTTONS, ENCODERS, WHITE},
    sinks::{self, Wav},
};
use std::{
    cell::UnsafeCell,
    ffi::CString,
    fs::{self, File},
    io::{self, BufWriter, Read, Write},
    path::PathBuf,
    time::Instant,
};

struct Simulator {
    clock: Clock,
    legacy: bool,
    peak: f32,
    wav: Option<Wav>,
    pcm: Option<BufWriter<File>>,
    render_ns: u128,
    frames: Option<File>,
    frame_number: u32,
    frame_start: u32,
    fps: u32,
    autopilot: bool,
    autopilot_time: u32,
    dir: String,
    // ui_restore_start retains this pointer across its fade/store callbacks.
    // Never move the allocation, expose references or free it while C runs.
    restore: Box<UnsafeCell<[u8; STORE_MAX]>>,
    // The upload API also retains its source until a render boundary.
    pack: Box<UnsafeCell<[u8; STORE_MAX]>>,
}
impl Drop for Simulator {
    fn drop(&mut self) {
        unsafe { abi::fm1_sim_restore_detach() };
    }
}
impl Simulator {
    fn new(dir: String) -> Self {
        Self {
            clock: Clock::default(),
            legacy: std::env::var_os("AMB_LEGACY_BOOT").is_some(),
            peak: 0.0,
            wav: None,
            pcm: std::env::var_os("FM1_SIM_PCM")
                .and_then(|path| File::create(path).ok())
                .map(BufWriter::new),
            render_ns: 0,
            frames: None,
            frame_number: 0,
            frame_start: 0,
            fps: 30,
            autopilot: false,
            autopilot_time: 0,
            dir,
            restore: Box::new(UnsafeCell::new([0; STORE_MAX])),
            pack: Box::new(UnsafeCell::new([0; STORE_MAX])),
        }
    }
    fn path(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("{}/{}", self.dir, name))
    }
    fn boot(&mut self) {
        let seed = platform(|p| p.seed);
        unsafe { abi::fm1_sim_boot(seed, self.legacy as u32) };
    }
    fn run(&mut self, ms: u32) {
        let mut block = [0i32; 512];
        for _ in 0..ms {
            self.clock.tick();
            platform(|p| p.now = self.clock.now);
            unsafe { abi::fm1_sim_service() };
            while self.clock.audio_ready() {
                let master = platform(|p| p.master);
                let start = Instant::now();
                unsafe { abi::en_render(block.as_mut_ptr(), 256, master) };
                self.render_ns += start.elapsed().as_nanos();
                if let Some(file) = &mut self.pcm {
                    let mut bytes = [0u8; 2048];
                    for (word, part) in block.iter().zip(bytes.chunks_exact_mut(4)) {
                        part.copy_from_slice(&word.to_le_bytes());
                    }
                    let _ = file.write_all(&bytes);
                }
                let pcm = sinks::pcm(&block, &mut self.peak);
                if let Some(w) = &mut self.wav {
                    let _ = w.append(&pcm);
                }
                self.clock.rendered();
            }
            if self.clock.frame_ready() {
                if self.autopilot {
                    self.steer();
                }
                unsafe { abi::ui_frame() };
            } else {
                unsafe { abi::ui_input_only() };
            }
        }
    }
    fn steer(&mut self) {
        let mut r = Runner::default();
        unsafe { abi::fm1_sim_runner(&mut r) };
        if r.on == 0 || r.over != 0 || self.clock.now.wrapping_sub(self.autopilot_time) < 110 {
            return;
        }
        let (mut best, mut target, mut danger, mut dir) = (99.0f32, -1, false, 0);
        for row in &r.rows {
            let z = row.beat - r.beat;
            if row.done != 0 || !(0.0..=4.0).contains(&z) {
                continue;
            }
            if (row.wall >> r.lane) & 1 != 0 && z < 1.8 {
                danger = true;
            }
            for l in 0..8 {
                if (row.gate >> l) & 1 != 0 && z < best && z > 0.4 {
                    best = z;
                    target = l;
                }
            }
        }
        if danger {
            for d in 1..4 {
                if dir != 0 {
                    break;
                }
                for s in [1, -1] {
                    let lane = (r.lane as i32 + s * d + 8) % 8;
                    let mut ok = true;
                    for row in &r.rows {
                        let z = row.beat - r.beat;
                        if row.done == 0 && (0.0..2.2).contains(&z) && (row.wall >> lane) & 1 != 0 {
                            ok = false;
                        }
                    }
                    if ok {
                        dir = s;
                        break;
                    }
                }
            }
        } else if target >= 0 && target != r.lane as i32 {
            let d = (target - r.lane as i32 + 8) % 8;
            dir = if d <= 4 { 1 } else { -1 };
        }
        if dir != 0 {
            platform(|p| p.enc[0] = p.enc[0].wrapping_add(dir));
            self.autopilot_time = self.clock.now;
        }
    }
    fn shot(&self, name: &str) {
        let bytes = platform(|p| sinks::png(&p.framebuffer));
        let path = self.path(name);
        if fs::write(&path, bytes).is_err() {
            eprintln!("cannot write {}", path.display());
        }
    }
    fn expect(&self, name: &str, value: &str, quiet: bool) -> i32 {
        if name == "peak_db_max" || name == "peak_db_min" {
            let db = 20.0 * (self.peak.max(1e-9) as f64).log10();
            let limit = value.parse::<f64>().unwrap_or(0.0);
            let bad = if name == "peak_db_max" {
                db > limit
            } else {
                db < limit
            };
            println!(
                "{} peak {:.1} dBFS ({} {})",
                if bad { "FAIL" } else { "  ok" },
                db,
                &name[8..],
                value
            );
            return bad as i32;
        }
        let name_c = CString::new(name).unwrap();
        let mut got = 0;
        if unsafe { abi::fm1_sim_observe(name_c.as_ptr().cast(), &mut got) } == 0 {
            if !name.starts_with("jam_") {
                println!("FAIL unknown expect {name}");
            }
            return 1;
        }
        let comparison = Comparison::parse(value);
        let bad = !comparison.accepts(got);
        if !quiet {
            match (bad, comparison) {
                (true, Comparison::Equal(_)) => {
                    println!("FAIL expect {name} == {value}, got {got}")
                }
                (true, _) => println!("FAIL expect {name} {value}, got {got}"),
                (false, Comparison::Equal(_)) => println!("  ok {name} == {got}"),
                (false, _) => println!("  ok {name} == {got} ({value})"),
            }
        }
        bad as i32
    }
    fn restore_object(&mut self, object: u32, mode: &str) {
        let ptr = self.restore.get().cast::<u8>();
        // Only raw access to the address-stable retained scratch. C may retain
        // the pointer, but runs synchronously and never reads during this copy.
        let n = platform(|p| {
            p.store.load(object, unsafe {
                core::slice::from_raw_parts_mut(ptr, STORE_MAX)
            })
        });
        let mut metadata = [0u32; 16];
        unsafe { abi::fm1_sim_layout(metadata.as_mut_ptr()) };
        let mut m = n;
        let mut tmp = [0u8; STORE_MAX];
        unsafe { core::ptr::copy_nonoverlapping(ptr, tmp.as_mut_ptr(), STORE_MAX) };
        let word = |b: &[u8]| u32::from_le_bytes(b[..4].try_into().unwrap());
        if mode.starts_with('b') && n > 0 {
            tmp[0] ^= 255;
        }
        if mode.starts_with('s') && n > 0 && word(&tmp) == metadata[0] {
            let saved = u16::from_le_bytes([tmp[8], tmp[9]]);
            if let Some(i) = (0..10).find(|i| saved >> i & 1 != 0) {
                let at = metadata[1] + i * metadata[2] + metadata[3] + metadata[4];
                tmp[at as usize] = metadata[5] as u8;
            }
        }
        if mode.starts_with('o') && n > 0 && word(&tmp) == metadata[0] {
            tmp[14] = 0;
            m = metadata[6] as i32;
        } else if mode.starts_with('o') && object == metadata[7] && n == metadata[8] as i32 {
            m = metadata[9] as i32;
        } else if mode.starts_with('o') && n == metadata[10] as i32 {
            let old = metadata[11] as usize - 3;
            let mut out = [0u8; STORE_MAX];
            out[..8 + old].copy_from_slice(&tmp[..8 + old]);
            out[4..6].copy_from_slice(&(old as u16).to_le_bytes());
            let mut q = 8 + old;
            out[q..q + 16].copy_from_slice(&tmp[metadata[12] as usize..metadata[12] as usize + 16]);
            q += 16;
            q += q & 1;
            out[q..q + 4].copy_from_slice(&tmp[metadata[13] as usize..metadata[13] as usize + 4]);
            q += 4;
            tmp = out;
            m = q as i32;
        }
        unsafe { core::ptr::copy_nonoverlapping(tmp.as_ptr(), ptr, STORE_MAX) };
        let valid = m > 0 && unsafe { abi::ui_restore_valid(object, ptr, m as u32) } != 0;
        platform(|p| p.restore_valid = valid as i32);
        if valid {
            unsafe { abi::ui_restore_start(object, ptr, m as u32) };
        }
    }
    fn old_bank(&self, piece: i32, pad: i32) {
        let mut metadata = [0; 16];
        unsafe { abi::fm1_sim_layout(metadata.as_mut_ptr()) };
        let mut bank = [0u8; STORE_MAX];
        let mut params = vec![0i16; metadata[14] as usize];
        let mut pats = [0u16; 36];
        let mut pool = 0u16;
        bank[..4].copy_from_slice(b"HYP2");
        bank[4] = 1;
        bank[8] = 76;
        bank[10..12].copy_from_slice(&(1u16.wrapping_shl(piece as u32)).to_le_bytes());
        bank[14] = 1;
        bank[16] = 2;
        let mut o = 20;
        for i in 0..10 {
            unsafe { abi::fm1_sim_recipe(i, params.as_mut_ptr(), &mut pool, pats.as_mut_ptr()) };
            if i == piece {
                params[metadata[15] as usize] = pad as i16;
            }
            for v in params[..76]
                .iter()
                .map(|v| *v as u16)
                .chain(core::iter::once(pool))
                .chain(pats)
            {
                bank[o..o + 2].copy_from_slice(&v.to_le_bytes());
                o += 2;
            }
        }
        platform(|p| p.store.save(0, &bank[..o + 16], false));
    }
    fn fuzz(&self, seed: u32) {
        let mut r = seed.wrapping_mul(2654435761).wrapping_add(1);
        fn next(r: &mut u32) -> u32 {
            *r ^= *r << 13;
            *r ^= *r >> 17;
            *r ^= *r << 5;
            *r
        }
        let n = (next(&mut r) % 3841) as usize;
        let mut b = [0u8; STORE_MAX];
        for v in &mut b[..n] {
            *v = next(&mut r) as u8;
        }
        if n >= 20 && next(&mut r) & 3 != 0 {
            b[..4].copy_from_slice(b"HYP2");
            b[4] = 1;
            b[5..8].fill(0);
            if next(&mut r) & 1 != 0 {
                b[8] = (next(&mut r) % 100) as u8;
                b[9] = 0;
            }
            if next(&mut r) & 1 != 0 {
                for k in (20..n.saturating_sub(1)).step_by(2) {
                    if next(&mut r) & 7 != 0 {
                        b[k + 1] = 0;
                    }
                }
            }
        } else if n >= 4 && next(&mut r) & 1 != 0 {
            b[..4].copy_from_slice(b"HYP1");
        }
        platform(|p| p.store.save(0, &b[..n], false));
    }
    fn close_wav(&mut self) {
        if let Some(w) = self.wav.take() {
            let _ = w.finish();
        }
    }
    fn command(&mut self, line: &[u8], lineno: usize) -> Result<i32, i32> {
        let [cmd, a, b, c] = scenario::fields(line);
        let num = |s: &str| scenario::number(s, 10);
        let mut failures = 0;
        if cmd.is_empty() || line.first() == Some(&b'#') {
            return Ok(0);
        }
        match cmd.as_str() {
            "wait" => self.run(num(&a) as u32),
            "press" | "release" | "tap" => {
                let Some(i) = BUTTONS.iter().position(|s| *s == a) else {
                    println!("line {lineno}: no button {a}");
                    return Err(2);
                };
                if cmd != "release" {
                    platform(|p| p.buttons |= 1 << i);
                }
                if cmd == "tap" {
                    self.run(40);
                }
                if cmd != "press" {
                    platform(|p| p.buttons &= !(1 << i));
                }
                self.run(if cmd == "tap" { 40 } else { 2 });
            }
            "key" | "tapkey" => {
                let k = scenario::key(&a) as u32;
                let bit = 1u32.wrapping_shl(k);
                if cmd == "tapkey" {
                    platform(|p| p.keys |= bit);
                    self.run(60);
                    platform(|p| p.keys &= !bit);
                    self.run(30);
                } else {
                    platform(|p| {
                        if b == "down" {
                            p.keys |= bit
                        } else {
                            p.keys &= !bit
                        }
                    });
                    self.run(2);
                }
            }
            "strum" => {
                let (mut x, y, gap) = (num(&a), num(&b), if c.is_empty() { 25 } else { num(&c) });
                let step = if y >= x { 1 } else { -1 };
                loop {
                    let bit = 1 << WHITE[(x & 15) as usize];
                    platform(|p| p.keys |= bit);
                    self.run(gap as u32);
                    platform(|p| p.keys &= !bit);
                    if x == y {
                        break;
                    }
                    x += step;
                }
                self.run(2);
            }
            "turn" | "spin" => {
                let Some(i) = ENCODERS.iter().position(|s| *s == a) else {
                    println!("line {lineno}: no encoder {a}");
                    return Err(2);
                };
                let n = num(&b);
                if cmd == "spin" {
                    platform(|p| p.enc[i] = p.enc[i].wrapping_add(n));
                    unsafe { abi::fm1_sim_accel(1) };
                    self.run(1);
                    unsafe { abi::fm1_sim_accel(0) };
                } else {
                    for _ in 0..n.unsigned_abs() {
                        platform(|p| p.enc[i] += if n > 0 { 1 } else { -1 });
                        self.run(if c.is_empty() { 80 } else { num(&c) as u32 });
                    }
                }
            }
            "master" => {
                platform(|p| p.master = num(&a) as u32);
            }
            "midi" => {
                let s = scenario::number(&a, 16) as u32;
                let packet = (if s >= 0xf0 { 15 } else { s >> 4 })
                    | s << 8
                    | (scenario::number(&b, 16) as u32) << 16
                    | (scenario::number(&c, 16) as u32) << 24;
                platform(|p| p.enqueue(packet));
            }
            "wav" => {
                self.wav = Wav::open(&self.path(&a)).ok();
            }
            "wavstop" => self.close_wav(),
            "peakreset" => self.peak = 0.0,
            "shot" => self.shot(&a),
            "frames" => {
                if !b.is_empty() {
                    self.fps = num(&b) as u32;
                }
                if self.frames.is_none() {
                    self.frames = File::create(self.path("frames.txt")).ok();
                    self.frame_start = self.clock.now;
                }
                for _ in 0..num(&a) {
                    let t = self
                        .frame_start
                        .wrapping_add((self.frame_number as u64 * 1000 / self.fps as u64) as u32);
                    if t > self.clock.now {
                        self.run(t - self.clock.now);
                    }
                    self.shot(&format!("frame_{:05}.png", self.frame_number));
                    let (lb, lk, hb, hk) =
                        platform(|p| (p.lit_buttons, p.lit_keys, p.buttons, p.keys));
                    if let Some(f) = &mut self.frames {
                        let _ = writeln!(f, "{} {lb} {lk} {hb} {hk}", self.frame_number);
                    }
                    self.frame_number = self.frame_number.wrapping_add(1);
                }
                if let Some(f) = &mut self.frames {
                    let _ = f.flush();
                }
            }
            "leds" => {
                let (buttons, keys) = platform(|p| (p.lit_buttons, p.lit_keys));
                print!("  leds: buttons");
                for (i, b) in BUTTONS.iter().enumerate() {
                    if buttons >> i & 1 != 0 {
                        print!(" {b}");
                    }
                }
                print!(" | white ");
                for i in WHITE {
                    print!("{}", if keys >> i & 1 != 0 { '#' } else { '.' });
                }
                print!(" black ");
                for i in BLACK {
                    print!("{}", if keys >> i & 1 != 0 { '#' } else { '.' });
                }
                println!();
            }
            "expect" => failures += self.expect(&a, &b, false),
            "waitfor" => {
                let mut t = 0;
                while t < (num(&c) as u32) && self.expect(&a, &b, true) != 0 {
                    self.run(10);
                    t += 10;
                }
                failures += self.expect(&a, &b, false);
            }
            "restore" => self.restore_object(num(&a) as u32, &b),
            "bank76" => self.old_bank(num(&a), num(&b)),
            "storefuzz" => self.fuzz(num(&a) as u32),
            "loadobj" | "dumpobj" => {
                if cmd == "loadobj" {
                    match read_prefix(&b) {
                        Ok(bytes) => {
                            platform(|p| p.store.save(num(&a) as u32, &bytes, false));
                        }
                        Err(_) => {
                            eprintln!("{cmd}: cannot open {b}");
                            failures += 1;
                        }
                    }
                } else {
                    match File::create(&b) {
                        Ok(mut file) => {
                            let mut bytes = [0; STORE_MAX];
                            let n = platform(|p| p.store.load(num(&a) as u32, &mut bytes));
                            if n > 0 {
                                let _ = file.write_all(&bytes[..n as usize]);
                            }
                        }
                        Err(_) => {
                            eprintln!("{cmd}: cannot open {b}");
                            failures += 1;
                        }
                    }
                }
            }
            "autopilot" => self.autopilot = num(&a) != 0,
            "splash" => unsafe { abi::fm1_sim_splash(num(&a) as u32) },
            "setpar" => unsafe { abi::ui_param_set(num(&a), num(&b)) },
            "oldimage" => {
                platform(|p| {
                    if p.store.lengths[0] > 13 {
                        p.store.bytes[0][13] = 0;
                    }
                });
            }
            "usbpower" => {
                platform(|p| p.usb = num(&a));
            }
            "seamless" => {
                platform(|p| p.seamless = num(&a));
            }
            "battery" => {
                platform(|p| p.battery = num(&a) as u32);
            }
            "seed" => {
                platform(|p| p.seed = num(&a) as u32);
            }
            "reboot" => {
                platform(|p| p.store.locked = false);
                unsafe { abi::fm1_sim_reset_project() };
                self.boot();
            }
            "jamboot" => {
                self.legacy = false;
                std::env::remove_var("AMB_LEGACY_BOOT");
                self.boot();
            }
            "jam" => {
                unsafe { abi::en_jam_set(num(&a), num(&b)) };
                self.run(40);
            }
            "jamstop" => {
                unsafe { abi::en_play(0) };
                self.run(40);
            }
            "jampack" => {
                let bytes = match read_prefix(&a) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("{a}: {}", io_message(&e));
                        return Err(2);
                    }
                };
                let ptr = self.pack.get().cast::<u8>();
                unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
                if unsafe { abi::en_jam_upload(ptr, bytes.len() as u32) } == 0 {
                    eprintln!("invalid jam pack");
                    return Err(2);
                }
                self.run(40);
            }
            "echo" => {
                print!(
                    "{}",
                    String::from_utf8_lossy(line.get(5..).unwrap_or_default())
                );
            }
            _ => {
                println!("line {lineno}: unknown command {cmd}");
                return Err(2);
            }
        }
        Ok(failures)
    }
}
fn read_prefix(path: &str) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut bytes = Vec::with_capacity(STORE_MAX);
    file.take(STORE_MAX as u64).read_to_end(&mut bytes)?;
    Ok(bytes)
}
fn io_message(error: &io::Error) -> String {
    let text = error.to_string();
    text.split(" (os error ").next().unwrap_or(&text).to_owned()
}
fn main() {
    #[cfg(feature = "rust-engine")]
    std::hint::black_box(fm1_jam_bridge::fm1_rust_jam_stat(0));
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: fm1-sim SCRIPT [OUTDIR]");
        std::process::exit(2);
    }
    let script = match File::open(&args[1]) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: {}", args[1], io_message(&e));
            std::process::exit(2);
        }
    };
    let mut sim = Simulator::new(args.get(2).cloned().unwrap_or_else(|| ".".into()));
    abi::midi_file(File::create(sim.path("midi_out.txt")).ok());
    unsafe { abi::fm1_sim_splash(0) };
    sim.boot();
    let mut failures = 0;
    let mut status = 0;
    for (i, line) in scenario::records(script).enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                eprintln!("{}: {e}", args[1]);
                status = 2;
                break;
            }
        };
        match sim.command(&line, i + 1) {
            Ok(n) => failures += n,
            Err(n) => {
                status = n;
                break;
            }
        }
    }
    abi::midi_file(None);
    if status == 0 {
        sim.close_wav();
        if let Some(path) = std::env::var_os("FM1_SIM_TRACE") {
            let bytes = platform(|p| p.trace(sim.clock.audio_due, sim.clock.ui_due, sim.peak));
            fs::write(path, bytes).expect("write deterministic simulator evidence");
        }
        let budget = sim.clock.renders as f64 * 256.0 * 1e9 / 44100.0;
        println!(
            "host: {} ms simulated, {} blits, render {:.1}% of real time (host CPU){}",
            sim.clock.now,
            platform(|p| p.blits),
            if budget > 0.0 {
                100.0 * sim.render_ns as f64 / budget
            } else {
                0.0
            },
            if failures != 0 {
                ", EXPECTATIONS FAILED"
            } else {
                ""
            }
        );
        status = if failures != 0 { 1 } else { 0 };
    }
    drop(sim);
    std::process::exit(status);
}
