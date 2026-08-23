extern crate napi_build;

fn main() {
    napi_build::setup();

    #[cfg(target_env = "msvc")]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            println!("cargo:rustc-link-lib=advapi32");
        }
    }
}
