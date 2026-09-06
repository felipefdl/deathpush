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
    Ok(output) if output.status.success() => shrink_licenses(&output.stdout),
    _ => String::from("[]"),
  };
  std::fs::write(&licenses_path, json).expect("write licenses.json");
  println!("cargo:rerun-if-changed=../../Cargo.lock");
  println!("cargo:rerun-if-changed=../../.git/HEAD");
  println!("cargo:rerun-if-changed=../../.git/refs");
}

fn shrink_licenses(stdout: &[u8]) -> String {
  let Ok(metadata) = serde_json::from_slice::<serde_json::Value>(stdout) else {
    return String::from("[]");
  };
  let members: Vec<&str> = metadata["workspace_members"]
    .as_array()
    .map(|arr| arr.iter().filter_map(|value| value.as_str()).collect())
    .unwrap_or_default();
  let Some(packages) = metadata["packages"].as_array() else {
    return String::from("[]");
  };
  let rows: Vec<serde_json::Value> = packages
    .iter()
    .filter(|package| package["id"].as_str().map(|id| !members.contains(&id)).unwrap_or(true))
    .map(|package| {
      serde_json::json!({
        "name": package["name"],
        "license": package["license"],
        "repository": package["repository"],
      })
    })
    .collect();
  serde_json::to_string(&rows).unwrap_or_else(|_| String::from("[]"))
}
