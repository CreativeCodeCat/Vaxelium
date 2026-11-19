//! Builds the web client: the desktop client compiled to WebAssembly, with the files the chat server
//! serves from `server/web/dist`. Needs the wasm32 target, and `wasm-bindgen` at the version in Cargo.lock
//! (`cargo install wasm-bindgen-cli`). `wasm-opt` (binaryen) makes the WebAssembly smaller when it is
//! installed. Compressed copies (`.gz`) are made too, which the chat server sends to browsers that accept them.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::run;

pub fn build(root: &Path) -> Result<(), String> {
    let client = root.join("client");
    if !client.join("Cargo.toml").is_file() {
        return Err("The client is not here. Clone its repository into client/ first.".into());
    }
    let bindgen = find_program("wasm-bindgen")
        .or_else(|| home_cargo_bin("wasm-bindgen"))
        .ok_or("wasm-bindgen is not installed. Install it with: cargo install wasm-bindgen-cli")?;

    // The client is a separate repository, so it is built in its own folder.
    run(&client, "cargo", ["build", "--profile", "web", "--target", "wasm32-unknown-unknown"])?;

    let out = root.join("server/web/dist");
    let wasm = client.join("target/wasm32-unknown-unknown/web/client.wasm");
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    run(
        root,
        bindgen,
        [
            OsString::from("--target"),
            "web".into(),
            "--no-typescript".into(),
            "--out-dir".into(),
            out.clone().into(),
            "--out-name".into(),
            "client".into(),
            wasm.into(),
        ],
    )?;

    let bg = out.join("client_bg.wasm");
    if let Some(opt) = find_program("wasm-opt") {
        run(
            root,
            opt,
            [
                OsString::from("-Oz"),
                "--enable-bulk-memory".into(),
                "--enable-nontrapping-float-to-int".into(),
                "-o".into(),
                bg.clone().into(),
                bg.clone().into(),
            ],
        )?;
    }

    fs::copy(root.join("server/web/index.html"), out.join("index.html")).map_err(|e| e.to_string())?;
    // Compressed copies, which the server sends to browsers that accept them. The originals are kept.
    gzip_keep(&bg)?;
    gzip_keep(&out.join("client.js"))?;
    // Emoji artwork is fetched as it is needed, so it is served as files rather than built in.
    copy_dir(&client.join("ui/emoji"), &out.join("emoji"))?;

    let gz = out.join("client_bg.wasm.gz");
    println!(
        "Built the web client in {}: {} of WebAssembly, {} compressed.",
        out.display(),
        human_size(&bg),
        human_size(&gz)
    );
    Ok(())
}

/// Writes `<file>.gz` next to the file, at the highest compression, replacing an earlier copy.
fn gzip_keep(path: &Path) -> Result<(), String> {
    let data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let gz_path = PathBuf::from(format!("{}.gz", path.display()));
    let file = File::create(&gz_path).map_err(|e| format!("{}: {e}", gz_path.display()))?;
    let mut encoder = GzEncoder::new(file, Compression::best());
    encoder.write_all(&data).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())?;
    Ok(())
}

/// The first program of this name on the PATH, if there is one.
fn find_program(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// A program in the Cargo bin folder, where `cargo install` puts it.
fn home_cargo_bin(name: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let candidate = PathBuf::from(home).join(".cargo/bin").join(name);
    candidate.is_file().then_some(candidate)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// A file's size as a short figure, such as "1.2 MB" or "310 KB".
fn human_size(path: &Path) -> String {
    let bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0) as f64;
    if bytes >= 1024.0 * 1024.0 {
        format!("{:.1} MB", bytes / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KB", bytes / 1024.0)
    }
}

