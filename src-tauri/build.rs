fn main() {
    tauri_build::build();
    // macOS system dictation (Apple Speech), bridged from Objective-C; ported from Coffee CLI.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=src/voice/apple_speech.m");
        cc::Build::new().file("src/voice/apple_speech.m").flag("-fobjc-arc").flag("-fblocks").compile("wings_apple_speech");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=objc");
        println!("cargo:rustc-link-arg=-Wl,-weak_framework,Speech");
    }
}
