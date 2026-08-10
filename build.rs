#[allow(dead_code)]
#[path = "src/icon/render.rs"]
mod render;

#[allow(dead_code)]
#[path = "src/icon/ico.rs"]
mod ico;

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/icon/render.rs");
    println!("cargo:rerun-if-changed=src/icon/ico.rs");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let images: Vec<(u32, Vec<u8>)> = ico::SIZES
        .iter()
        .map(|size| (*size, render::rgba(render::IconState::Idle, 0, *size)))
        .collect();

    let path =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo")).join("app.ico");
    std::fs::write(&path, ico::encode(&images)).expect("the icon should be writable");

    winresource::WindowsResource::new()
        .set_icon(path.to_str().expect("the icon path should be valid UTF-8"))
        .compile()
        .expect("the resource should compile");
}
