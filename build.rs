use std::env;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=vendor/CMakeLists.txt");
    println!("cargo:rerun-if-changed=vendor/include/whisper.h");
    println!("cargo:rerun-if-changed=vendor/src/whisper.cpp");
    println!("cargo:rerun-if-changed=vendor/ggml/CMakeLists.txt");

    // Require a system OpenBLAS installation and capture its linker metadata.
    let openblas = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("openblas64")
        .or_else(|_| pkg_config::Config::new().cargo_metadata(false).probe("openblas"))
        .expect("system OpenBLAS not found via pkg-config (openblas64/openblas)");

    let mut cfg = cmake::Config::new("vendor");
    cfg.profile(
        &env::var("VOXPIPE_WHISPER_CMAKE_BUILD_TYPE")
            .unwrap_or_else(|_| "Release".to_string()),
    );

    cfg.define("BUILD_SHARED_LIBS", "ON")
        .define("WHISPER_BUILD_TESTS", "OFF")
        .define("WHISPER_BUILD_EXAMPLES", "OFF")
        .define("WHISPER_BUILD_SERVER", "OFF")
        .define("WHISPER_CURL", "OFF")
        .define("GGML_BLAS", "ON")
        .define("GGML_BLAS_VENDOR", "OpenBLAS")
        .define("BLA_VENDOR", "OpenBLAS");

    let dst = cfg.build();
    let lib_dir = dst.join("lib");
    let lib64_dir = dst.join("lib64");

    if Path::new(&lib_dir).exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
    }
    if Path::new(&lib64_dir).exists() {
        println!("cargo:rustc-link-search=native={}", lib64_dir.display());
    }

    println!("cargo:rustc-link-lib=dylib=whisper");

    for path in &openblas.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for lib in &openblas.libs {
        println!("cargo:rustc-link-lib={lib}");
    }
}
