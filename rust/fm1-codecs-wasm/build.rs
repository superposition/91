fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=--initial-memory=4194304");
        println!("cargo:rustc-link-arg=--max-memory=4194304");
        println!("cargo:rustc-link-arg=-zstack-size=524288");
    }
}
