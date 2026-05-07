use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn pad4(len: usize) -> usize {
    (len + 3) & !3
}

fn pad_file(name: &str, out_dir: &Path) {
    let src = Path::new("firmware").join(name);
    let dst = out_dir.join(name);

    let mut data = fs::read(&src).unwrap();
    data.resize(pad4(data.len()), 0);

    fs::write(&dst, data).unwrap();

    println!("cargo:rerun-if-changed={}", src.display());
}

#[allow(dead_code)]
fn copy_file(name: &str, out_dir: &Path) {
    let src = Path::new("firmware").join(name);
    let dst = out_dir.join(name);

    fs::copy(&src, &dst).unwrap();

    println!("cargo:rerun-if-changed={}", src.display());
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    pad_file("w43439A0_7_95_49_00_combined.bin", &out_dir);

    pad_file("wifi_nvram_43439.bin", &out_dir);
}
