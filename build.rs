use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=vendor/CMakeLists.txt");
    println!("cargo:rerun-if-changed=vendor/include/whisper.h");
    println!("cargo:rerun-if-changed=vendor/src/whisper.cpp");
    println!("cargo:rerun-if-changed=vendor/ggml/CMakeLists.txt");
    println!("cargo:rerun-if-changed=src/whisper_sys/wrapper.h");

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
        .define("CMAKE_BUILD_WITH_INSTALL_RPATH", "ON")
        .define("CMAKE_INSTALL_RPATH", "$ORIGIN")
        .define("GGML_BLAS", "ON")
        .define("GGML_BLAS_VENDOR", "OpenBLAS")
        .define("BLA_VENDOR", "OpenBLAS");

    let dst = cfg.build();
    let lib_dir = dst.join("lib");
    let lib64_dir = dst.join("lib64");

    if Path::new(&lib_dir).exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-arg=-Wl,--disable-new-dtags");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }
    if Path::new(&lib64_dir).exists() {
        println!("cargo:rustc-link-search=native={}", lib64_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib64_dir.display());
    }

    println!("cargo:rustc-link-lib=dylib=whisper");

    for path in &openblas.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for lib in &openblas.libs {
        println!("cargo:rustc-link-lib={lib}");
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let installed_include = dst.join("include");
    let bindings = bindgen::Builder::default()
        .header("src/whisper_sys/wrapper.h")
        .clang_arg("-x")
        .clang_arg("c")
        .clang_arg("-std=c11")
        .clang_arg("-Ivendor/include")
        .clang_arg("-Ivendor/ggml/include")
        .clang_arg(format!("-I{}", installed_include.display()))
        .allowlist_function("(whisper|ggml)_.*")
        .allowlist_type("(whisper|ggml)_.*")
        .allowlist_var("(WHISPER|GGML)_.*")
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("unable to generate whisper bindings");

    let bindings_path = out_dir.join("bindings.rs");
    bindings
        .write_to_file(&bindings_path)
        .expect("unable to write whisper bindings");

    // Rust 2024 requires `unsafe extern` blocks.
    let generated = fs::read_to_string(&bindings_path)
        .expect("unable to read generated whisper bindings");
    let patched = generated.replace("extern \"C\" {", "unsafe extern \"C\" {");
    fs::write(&bindings_path, patched).expect("unable to patch generated whisper bindings");
}
