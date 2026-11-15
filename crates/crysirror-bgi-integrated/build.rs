// Platform cfg aliases for file access and desktop text detection.
#[cfg(feature = "vita-gxm")]
#[path = "../../platform/psv/native_build.rs"]
mod vita_build;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(fd_policy)");
    println!("cargo::rustc-check-cfg=cfg(desktop_host)");
    println!("cargo::rustc-check-cfg=cfg(hosted_files)");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    // Keep startup text detection out of mobile, browser and console builds.
    // Match the desktop-only chardetng dependency in Cargo.toml.
    if matches!(
        os.as_str(),
        "windows"
            | "macos"
            | "linux"
            | "freebsd"
            | "openbsd"
            | "netbsd"
            | "dragonfly"
            | "illumos"
            | "solaris"
            | "haiku"
    ) {
        println!("cargo::rustc-cfg=desktop_host");
    }
    let unix = std::env::var_os("CARGO_CFG_UNIX").is_some();
    // The consoles are `unix` to Rust but have no directory descriptors;
    // like the web, they hand the engine their file roots instead.
    let hosted = arch == "wasm32" || os == "vita" || os == "horizon";
    if hosted {
        println!("cargo::rustc-cfg=hosted_files");
    } else if unix {
        println!("cargo::rustc-cfg=fd_policy");
    }
    #[cfg(feature = "vita-gxm")]
    if os == "vita" {
        build_vita_gpu();
    }
}

#[cfg(feature = "vita-gxm")]
fn build_vita_gpu() {
    use std::{fmt::Write, path::PathBuf};
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let mut header = String::new();
    for name in ["emote_v", "emote_f"] {
        let path = format!("native/vita/shaders/{name}.gxp");
        println!("cargo:rerun-if-changed={path}");
        write!(header, "static const unsigned char {name}[] __attribute__((aligned(16))) = {{").unwrap();
        for byte in std::fs::read(path).expect("embedded GXP") {
            write!(header, "{byte},").unwrap();
        }
        header.push_str("};\n");
    }
    std::fs::write(out.join("shaders.h"), header).expect("generated GXP header");
    println!("cargo:rerun-if-changed=../../platform/psv/native_build.rs");
    println!("cargo:rerun-if-changed=native/vita/ethornell_console.h");
    vita_build::compile("crysirror_bgi_vita_gpu", &["native/vita/gpu.c"], &["native/vita".into(), out]);
}
