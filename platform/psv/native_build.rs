//! Shared VitaSDK C build wiring for retained integrated-runtime GXM drivers.
//! Included by build scripts; never initializes a device or compiles Cg.
use std::{env, path::PathBuf, process::Command};

pub fn compile(name: &str, sources: &[&str], includes: &[PathBuf]) {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("vita") {
        return;
    }
    println!("cargo:rerun-if-env-changed=VITASDK");
    let sdk = PathBuf::from(env::var_os("VITASDK").unwrap_or_else(|| "/usr/local/vitasdk".into()));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let mut objects = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        println!("cargo:rerun-if-changed={source}");
        let object = out.join(format!("{name}-{index}.o"));
        let mut command = Command::new(sdk.join("bin/arm-vita-eabi-gcc"));
        command.args(["-c", "-O2", "-std=gnu17", "-ffunction-sections", "-fdata-sections"])
            .arg("-I").arg(sdk.join("arm-vita-eabi/include"));
        for include in includes {
            command.arg("-I").arg(include);
        }
        command.arg(source).arg("-o").arg(&object);
        assert!(command.status().expect("start VitaSDK C compiler").success(), "VitaSDK compilation failed: {source}");
        objects.push(object);
    }
    let archive = out.join(format!("lib{name}.a"));
    if archive.exists() {
        std::fs::remove_file(&archive).expect("replace generated archive");
    }
    assert!(Command::new(sdk.join("bin/arm-vita-eabi-ar"))
        .arg("crs").arg(archive).args(objects).status()
        .expect("start VitaSDK archiver").success(), "VitaSDK archive failed");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-search=native={}", sdk.join("arm-vita-eabi/lib").display());
    println!("cargo:rustc-link-lib=static={name}");
    for library in ["vita2d", "SceGxm_stub", "SceDisplay_stub", "SceSysmodule_stub",
        "SceCommonDialog_stub", "SceAppUtil_stub", "SceAppMgr_stub", "SceSysmem_stub",
        "SceLibKernel_stub", "SceKernelThreadMgr_stub", "m"] {
        println!("cargo:rustc-link-lib=static={library}");
    }
}
