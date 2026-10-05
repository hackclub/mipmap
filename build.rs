use sha2::{Digest, Sha256};
use std::{env, fs, path::Path, process::Command};

const MODEL_URL: &str = "https://github.com/danielgatis/rembg/releases/download/v0.0.0/silueta.onnx";
const MODEL_SHA256: &str = "75da6c8d2f8096ec743d071951be73b4a8bc7b3e51d9a6625d63644f90ffeedb";

fn main() {
    for path in [
        "frontend/src",
        "frontend/static",
        "frontend/package.json",
        "frontend/package-lock.json",
        "frontend/tsconfig.json",
        "frontend/vite.config.ts",
        "migrations",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-env-changed=VITE_SITE_URL");

    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir("frontend")
        .status()
        .expect("failed to run npm; is Node installed?");
    assert!(status.success(), "frontend build failed");

    let model = Path::new(&env::var("OUT_DIR").unwrap()).join("silueta.onnx");
    if !matches(&model) {
        let status = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(&model)
            .arg(MODEL_URL)
            .status()
            .expect("failed to run curl; is it installed?");
        assert!(status.success(), "downloading the background removal model failed");
        assert!(matches(&model), "the background removal model has the wrong checksum");
    }
}

fn matches(path: &Path) -> bool {
    fs::read(path).is_ok_and(|bytes| hex::encode(Sha256::digest(bytes)) == MODEL_SHA256)
}
