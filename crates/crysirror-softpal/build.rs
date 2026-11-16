#[cfg(feature = "vita-gxm")]
#[path = "../../platform/psv/native_build.rs"]
mod vita_build;

fn main() {
    #[cfg(feature = "vita-gxm")]
    {
        println!("cargo:rerun-if-changed=../../platform/psv/native_build.rs");
        for header in ["native/vita/source/scene_gpu.h", "native/vita/source/cpu_memory.h",
            "native/common/console.h", "native/common/sena.h"] {
            println!("cargo:rerun-if-changed={header}");
        }
        vita_build::compile("crysirror_softpal_vita_gpu",
            &["native/vita/source/scene_gpu.c", "native/vita/source/cpu_memory.c"],
            &["native/vita/source".into()]);
    }
}
