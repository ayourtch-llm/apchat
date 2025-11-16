use std::process::Command;
use std::path::Path;

fn main() {
    // Tell cargo to rerun this build script if any Rust source files change
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Determine output directory (../../web/pkg from this crate)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let web_pkg_dir = Path::new(&manifest_dir)
        .join("../../web/pkg")
        .canonicalize()
        .unwrap_or_else(|_| {
            // If path doesn't exist yet, construct it manually
            Path::new(&manifest_dir).join("../../web/pkg")
        });

    println!("cargo:warning=Building WASM module to {:?}", web_pkg_dir);

    // Check if wasm-pack is available
    let wasm_pack_check = Command::new("wasm-pack")
        .arg("--version")
        .output();

    if wasm_pack_check.is_err() {
        println!("cargo:warning=wasm-pack not found! Install with: cargo install wasm-pack");
        println!("cargo:warning=Skipping WASM build. Run ./build.sh manually.");
        return;
    }

    // Run wasm-pack build
    let status = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(web_pkg_dir.to_str().unwrap())
        .current_dir(&manifest_dir)
        .status();

    match status {
        Ok(status) if status.success() => {
            println!("cargo:warning=✓ WASM build complete!");
        }
        Ok(status) => {
            println!("cargo:warning=✗ WASM build failed with status: {}", status);
        }
        Err(e) => {
            println!("cargo:warning=✗ Failed to run wasm-pack: {}", e);
        }
    }
}
