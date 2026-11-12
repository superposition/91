// SPDX-License-Identifier: GPL-3.0-only
#[path = "../tests/support/bus.rs"]
mod bus;
use std::io::Write;
fn main() {
    let mut trace = std::env::args()
        .nth(1)
        .map(|p| std::fs::File::create(p).unwrap());
    for operation in 0..10 {
        for variant in 0..32 {
            let mut io = bus::Bus::new(variant);
            let result = bus::execute(&mut io, operation, variant);
            io.word(result);
            if let Some(f) = &mut trace {
                f.write_all(&io.trace).unwrap();
            }
            println!("{{\"operation\":{operation},\"variant\":{variant},\"result\":{result},\"events\":{},\"hash\":\"{:08x}\"}}",io.events,io.hash);
        }
    }
}
