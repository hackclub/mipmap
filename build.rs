use std::process::Command;

fn main() {
    for path in [
        "frontend/src",
        "frontend/static",
        "frontend/package.json",
        "frontend/package-lock.json",
        "frontend/tsconfig.json",
        "frontend/vite.config.ts",
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
}
