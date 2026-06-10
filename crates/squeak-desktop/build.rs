fn main() {
    // dist/ is embedded at build time; without this, editing the frontend
    // doesn't retrigger a rebuild and `cargo run` serves stale assets.
    println!("cargo:rerun-if-changed=dist");
    tauri_build::build();
}
