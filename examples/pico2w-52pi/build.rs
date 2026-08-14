use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    fs::copy(
        "../../platform/pico2w/linker/memory.x",
        out.join("memory.x"),
    )
    .unwrap();

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=../../platform/pico2w/linker/memory.x");
}
