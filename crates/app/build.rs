use std::process::Command;

fn main() {
  let hash = Command::new("git")
    .args(["rev-parse", "--short", "HEAD"])
    .output()
    .ok()
    .filter(|output| output.status.success())
    .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    .unwrap_or_else(|| "unknown".to_string());
  println!("cargo:rustc-env=DEATHPUSH_GIT_HASH={hash}");
  let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
  let licenses_path = std::path::Path::new(&out_dir).join("licenses.json");
  let metadata = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
    .args(["metadata", "--format-version", "1", "--locked"])
    .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    .output();
  let json = match metadata {
    Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).into_owned(),
    _ => String::from("{\"packages\":[],\"workspace_members\":[]}"),
  };
  std::fs::write(&licenses_path, json).expect("write licenses.json");
  println!("cargo:rerun-if-changed=../../Cargo.lock");
  println!("cargo:rerun-if-changed=../../.git/HEAD");
  println!("cargo:rerun-if-changed=../../.git/refs");
}
