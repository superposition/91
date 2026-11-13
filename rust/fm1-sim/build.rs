// SPDX-License-Identifier: GPL-3.0-only
use std::{env, fs, path::PathBuf, process::Command};
fn run(command: &mut Command) {
    let status = command.status().expect("start native app compiler");
    assert!(status.success(), "native app compiler failed: {command:?}");
}
fn main() {
    println!("cargo:rerun-if-env-changed=FM1_SIM_SOURCE");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=AR");
    // Nested Cargo invocations may inherit another build script's feature env.
    // Compile-time cfg describes this package's actual resolved feature graph.
    if !cfg!(feature = "c-app") {
        return;
    }
    let repo = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let source = env::var_os("FM1_SIM_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.clone());
    let source = if source.is_absolute() {
        source
    } else {
        repo.join(source)
    }
    .canonicalize()
    .unwrap();
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut includes = String::new();
    for (key, file) in [
        ("GFX", "gfx.c"),
        ("APP", "app/app.h"),
        ("PROJECT", "app/project.c"),
        ("SESSION", "app/jam_session.c"),
        ("UI", "app/ui.c"),
        ("USB", "app/jam_usb.h"),
    ] {
        let p = source.join("firmware/src").join(file);
        includes += &format!("#define SIM_{key} \"{}\"\n", p.display());
    }
    fs::write(out.join("sim_source.h"), includes).unwrap();
    run(Command::new("python3")
        .arg(repo.join("tools/gen_font.py"))
        .arg(out.join("felucca_font.h"))
        .current_dir(&repo));
    let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
    for (file, name) in [
        (repo.join("host/rust_sim_app.c"), "app"),
        (source.join("firmware/src/dsp/amb.c"), "engine"),
    ] {
        let mut c = Command::new(&cc);
        c.args([
            "-O2",
            "-ffp-contract=off",
            "-std=c99",
            "-D_POSIX_C_SOURCE=200809L",
            "-DEN_HOST",
            "-Wno-unused-function",
        ])
        .arg("-I")
        .arg(&out)
        .arg("-I")
        .arg(source.join("firmware/src"))
        .arg("-I")
        .arg(source.join("firmware/src/dsp"));
        if cfg!(feature = "rust-engine") {
            for flag in [
                "JAM",
                "COMMANDS",
                "HANDOFF",
                "TRANSPORT",
                "SCHEDULER",
                "MELODIC",
                "EFFECTS",
                "FACTORY",
                "VOICES",
                "FM6",
                "FORMATS",
                "WORKER",
                "STORAGE",
                "HARMONY",
                "APP",
            ] {
                c.arg(format!("-DEN_RUST_{flag}=1"));
            }
        }
        run(c
            .arg("-c")
            .arg(&file)
            .arg("-o")
            .arg(out.join(format!("{name}.o"))));
    }
    run(Command::new(env::var("AR").unwrap_or_else(|_| "ar".into()))
        .arg("crs")
        .arg(out.join("libfm1_sim_app.a"))
        .arg(out.join("app.o"))
        .arg(out.join("engine.o")));
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=fm1_sim_app");
    println!("cargo:rustc-link-lib=m");
    println!(
        "cargo:rerun-if-changed={}",
        repo.join("host/rust_sim_app.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source.join("firmware/src").display()
    );
}
