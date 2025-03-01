fn main() {
    println!("cargo:rerun-if-changed=webui");

    std::process::Command::new("bun")
        .arg("install")
        .current_dir("webui")
        .status()
        .expect("Failed to run bun install");

    std::process::Command::new("bun")
        .arg("run")
        .arg("build")
        .current_dir("webui")
        .status()
        .expect("Failed to run bun run build");
}