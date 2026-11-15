#[cfg(feature = "vita-gxm")]
#[path = "../../platform/psv/native_build.rs"]
mod vita_build;

fn main() {
    #[cfg(feature = "vita-gxm")]
    {
        println!("cargo:rerun-if-changed=../../platform/psv/native_build.rs");
        vita_build::compile("crysirror_rfvp_vita_gpu", &["native/vita/rfvp_psv_gpu.c"], &[]);
    }
}
