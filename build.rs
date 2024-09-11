fn main() {
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=dylib=mfuuid");
        println!("cargo:rustc-link-lib=dylib=strmiids");
    }
}
