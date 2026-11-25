fn main() {
    println!("cargo:rerun-if-changed=native/macos.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    cc::Build::new()
        .file("native/macos.m")
        .flag("-fobjc-arc")
        .compile("qcbuddy_macos");
    for framework in ["Foundation", "IOBluetooth", "CoreAudio"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
