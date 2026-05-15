fn main() {
    // Use CARGO_CFG_TARGET_OS (the *target* platform, not the *host*) so that
    // cross-compilation from macOS to Linux does not attempt to compile the
    // Objective-C ScreenCaptureKit wrapper.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "macos" {
        cc::Build::new()
            .file("src/display_stream.m")
            .flag("-fobjc-arc")
            .compile("display_stream");

        println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=IOSurface");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rerun-if-changed=src/display_stream.m");
    }
}
