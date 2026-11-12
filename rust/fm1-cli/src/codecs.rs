// SPDX-License-Identifier: GPL-3.0-only
//! Bounded stdio transport only; all codec work remains in the safe core.
use std::io::{self, Read, Write};
const CAPACITY: usize = 1_100_000;
fn reply(op: u32, input: &[u8], output: &mut [u8], stdout: &mut impl Write) -> io::Result<()> {
    let result = fm1_codecs_wasm::api::run(op, input, output)
        .map(|n| n as i32)
        .unwrap_or_else(fm1_codecs_wasm::api::error_code);
    stdout.write_all(&result.to_le_bytes())?;
    if result > 0 {
        stdout.write_all(&output[..result as usize])?;
    }
    stdout.flush()
}
pub fn once(op: u32) -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take((CAPACITY + 1) as u64)
        .read_to_end(&mut input)?;
    if input.len() > CAPACITY {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "codec input exceeds capacity",
        ));
    }
    let mut output = vec![0; CAPACITY];
    reply(op, &input, &mut output, &mut io::stdout().lock())
}
pub fn stream() -> io::Result<()> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    let mut input = Vec::new();
    let mut output = vec![0; CAPACITY];
    loop {
        let mut header = [0u8; 8];
        let n = stdin.read(&mut header[..1])?;
        if n == 0 {
            return Ok(());
        }
        stdin.read_exact(&mut header[1..])?;
        let op = u32::from_le_bytes(header[..4].try_into().unwrap());
        let len = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
        if len > CAPACITY {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "codec input exceeds capacity",
            ));
        }
        input.resize(len, 0);
        stdin.read_exact(&mut input)?;
        reply(op, &input, &mut output, &mut stdout)?;
    }
}
