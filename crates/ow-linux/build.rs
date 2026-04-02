fn main() {
    cc::Build::new()
        .file("csrc/namespace.c")
        .file("csrc/cgroup.c")
        .file("csrc/mount.c")
        .include("csrc/include")
        .warnings(true)
        .flag("-Wall")
        .flag("-Wextra")
        .compile("ow_linux_c");

    println!("cargo:rerun-if-changed=csrc/");
}
