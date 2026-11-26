// rust-embed needs the UI folder to exist at compile time. On a fresh clone (before build.ps1 has
// produced the WebAssembly bundle) create it, so `cargo test -p desktop` still compiles.
fn main() {
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/dist");
    if !dist.exists() {
        let _ = std::fs::create_dir_all(&dist);
    }
    println!("cargo:rerun-if-changed=../ui/dist");
}
