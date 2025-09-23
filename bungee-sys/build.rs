use std::path::PathBuf;

// -------------------------------------------------------------------------------------------------

fn main() {
    // set up cargo build environment
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=CFLAGS");
    println!("cargo:rerun-if-env-changed=CXXFLAGS");

    println!("cargo:rerun-if-changed=vendor/");
    println!("cargo:rerun-if-changed=vendor/bungee/");
    println!("cargo:rerun-if-changed=vendor/bungee/CMakeLists.txt");

    // build bungee C++ lib with cmake
    build_bungee();
    // build our C++ wrappers with cpp_build
    build_wrappers();
}

// -------------------------------------------------------------------------------------------------

fn build_bungee() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let target_triple = std::env::var("TARGET").unwrap();

    if target_triple.contains("android") {
        panic!("android builds are not (yet) supported");
    }

    // Seems Rust always links against the release version of the MSVC runtime,
    // even in debug builds. So force building cmake release configs here...
    let profile = match std::env::var("PROFILE").unwrap().as_str() {
        "release" => "Release",
        _ => "RelWithDebInfo",
    };

    let _ = cmake::Config::new("vendor/bungee")
        .profile(profile)
        .define("CMAKE_EXPORT_COMPILE_COMMANDS", "ON")
        .define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreaded")
        .define("BUNGEE_BUILD_SHARED_LIBRARY", "OFF")
        .build_target("bungee_library")
        .build();

    // link bungee C++ lib
    let build_dir = format!("{}", out_dir.clone().join("build").display());
    println!("cargo:rustc-link-search=native={build_dir}");
    println!("cargo:rustc-link-search=native={build_dir}/{profile}",);
    println!("cargo:rustc-link-search=native={build_dir}/submodules/kissfft");
    println!("cargo:rustc-link-search=native={build_dir}/submodules/kissfft/{profile}",);

    println!("cargo:rustc-link-lib=static=bungee");
    println!("cargo:rustc-link-lib=static=kissfft-float");
}

// -------------------------------------------------------------------------------------------------

fn build_wrappers() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/stream.rs");
    println!("cargo:rerun-if-changed=src/stretcher.rs");

    let cargo_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let bungee_dir = cargo_dir.clone().join("vendor").join("bungee");
    let mut config: cpp_build::Config = cc::Build::new()
        .static_crt(true) // see CMAKE_MSVC_RUNTIME_LIBRARY above
        .flag_if_supported("-std=c++17")
        .include(bungee_dir.clone().join("bungee"))
        .include(bungee_dir.clone().join("submodules"))
        .include(bungee_dir.clone().join("submodules").join("eigen"))
        .clone()
        .into();
    config.build("src/lib.rs");
}
