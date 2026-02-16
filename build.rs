use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=vendor/CMakeLists.txt");
    println!("cargo:rerun-if-changed=vendor/include/whisper.h");
    println!("cargo:rerun-if-changed=vendor/src/whisper.cpp");
    println!("cargo:rerun-if-changed=vendor/ggml/CMakeLists.txt");
    println!("cargo:rerun-if-changed=src/whisper_sys/wrapper.h");
    println!("cargo:rerun-if-env-changed=VOXPIPE_WHISPER_CMAKE_BUILD_TYPE");
    println!("cargo:rerun-if-env-changed=VOXPIPE_WHISPER_FFMPEG");
    println!("cargo:rerun-if-env-changed=VOXPIPE_WHISPER_BUILD_EXAMPLES");

    let ffmpeg_enabled = env_flag("VOXPIPE_WHISPER_FFMPEG").unwrap_or(false);
    let build_examples = env_flag("VOXPIPE_WHISPER_BUILD_EXAMPLES").unwrap_or(ffmpeg_enabled);

    if ffmpeg_enabled && !build_examples {
        panic!(
            "VOXPIPE_WHISPER_FFMPEG requires examples to be enabled.\n\
             Set VOXPIPE_WHISPER_BUILD_EXAMPLES=1 (or unset it to accept the default)."
        );
    }

    if ffmpeg_enabled {
        probe_ffmpeg_pkg_config();
        println!("cargo:warning=whisper.cpp FFmpeg decode support enabled");
    }

    // Require a system OpenBLAS installation and capture its linker metadata.
    let openblas = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("openblas64")
        .or_else(|_| {
            pkg_config::Config::new()
                .cargo_metadata(false)
                .probe("openblas")
        })
        .expect("system OpenBLAS not found via pkg-config (openblas64/openblas)");

    let mut cfg = cmake::Config::new("vendor");
    cfg.profile(
        &env::var("VOXPIPE_WHISPER_CMAKE_BUILD_TYPE").unwrap_or_else(|_| "Release".to_string()),
    );

    cfg.define("BUILD_SHARED_LIBS", "ON")
        .define("WHISPER_BUILD_TESTS", "OFF")
        .define("WHISPER_BUILD_EXAMPLES", on_off(build_examples))
        .define("WHISPER_BUILD_SERVER", "OFF")
        .define("WHISPER_CURL", "OFF")
        .define("WHISPER_FFMPEG", on_off(ffmpeg_enabled))
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
    let generated =
        fs::read_to_string(&bindings_path).expect("unable to read generated whisper bindings");
    let patched = generated.replace("extern \"C\" {", "unsafe extern \"C\" {");
    fs::write(&bindings_path, patched).expect("unable to patch generated whisper bindings");
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "ON" } else { "OFF" }
}

fn env_flag(name: &str) -> Option<bool> {
    let value = match env::var(name) {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => return None,
        Err(err) => panic!("failed reading {name}: {err}"),
    };

    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => panic!(
            "invalid value for {name}: {value:?}. Expected one of: 1/0, true/false, yes/no, on/off"
        ),
    }
}

fn probe_ffmpeg_pkg_config() {
    for pkg in ["libavcodec", "libavformat", "libavutil", "libswresample"] {
        if let Err(err) = pkg_config::Config::new().cargo_metadata(false).probe(pkg) {
            panic!(
                "VOXPIPE_WHISPER_FFMPEG=1 but {pkg} was not found via pkg-config: {err}\n\
                 Install FFmpeg development packages first:\n\
                   Debian/Ubuntu: sudo apt install libavcodec-dev libavformat-dev libavutil-dev libswresample-dev\n\
                   RHEL/Fedora:  sudo dnf install libavcodec-free-devel libavformat-free-devel libavutil-free-devel libswresample-free-devel\n\
                 Then rebuild."
            );
        }
    }
}
