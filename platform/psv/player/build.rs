// SPDX-License-Identifier: MPL-2.0
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("vita") {
        return;
    }
    // Published VitaSDK packages only; vita2d carries precompiled shaders,
    // so no runtime shader compiler module is needed on the device.
    for library in [
        "vita2d", "SceGxm_stub", "SceDisplay_stub", "SceSysmodule_stub", "SceCommonDialog_stub",
        "SceAppUtil_stub", "SceAppMgr_stub", "SceSysmem_stub", "m",
    ] {
        println!("cargo:rustc-link-lib=static={library}");
    }
    // Room for vita-elf-create's SCE tables before the data segment.
    println!("cargo:rustc-link-arg=-Wl,--defsym=__sce_headroom=0x4000");
}
