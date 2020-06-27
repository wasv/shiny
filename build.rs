extern crate wasm_bindgen_cli_support;

use std::process::Command;
use std::env;
use std::fs;
use std::path::Path;

use wasm_bindgen_cli_support::Bindgen;

fn main() {
    println!("cargo:rerun-if-changed=res/");
    println!("cargo:rerun-if-changed=ui/");

    let cargo = env::var_os("CARGO").unwrap();
    let target_dir = "target-wasm32";

    eprintln!("Building UI for wasm platform...");
    Command::new(&cargo).arg("rustc")
                       .args(&["--package","shiny-ui"])
                       .args(&["--target", "wasm32-unknown-unknown"])
                       .args(&["--target-dir", &target_dir])
                       .status().unwrap();

    eprintln!("Generating packed and wrapped wasm build...");
    let dest_dir = Path::new(&target_dir).join("shiny");
    let src_path = Path::new(&target_dir).join("wasm32-unknown-unknown").join("debug").join("shiny-ui.wasm");

    let mut b = Bindgen::new();
    b.input_path(&src_path).web(true).unwrap();
    b.generate(&dest_dir).unwrap();

    eprintln!("Copying HTML template to build directory...");
    let src_path = Path::new("res").join("index.html");
    let dest_path = Path::new(&dest_dir).join("index.html");
    fs::create_dir_all(
        &dest_dir
    ).unwrap();
    fs::copy(
        &src_path,
        &dest_path
    ).unwrap();
}

