use std::path::PathBuf;

fn main() {
    // set up build environment
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=CFLAGS");
    println!("cargo:rerun-if-env-changed=CXXFLAGS");
    println!("cargo:rerun-if-changed=vendor/");
    println!("cargo:rerun-if-changed=vendor/bungee/");
    println!("cargo:rerun-if-changed=vendor/bungee/CMakeLists.txt");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let target_triple = std::env::var("TARGET").unwrap();

    if target_triple.contains("android") {
        panic!("android builds are not (yet) supported");
    }

    // build
    let mut dst = cmake::Config::new("vendor/bungee");

    // It seems like that Rust always links against the release version of the MSVC
    // runtime, even in debug builds. So we always build release builds here...
    let profile = "Release";

    let _dst = dst
        .profile(profile)
        .define("CMAKE_EXPORT_COMPILE_COMMANDS", "ON")
        .define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreaded")
        .define("BUNGEE_BUILD_SHARED_LIBRARY", "OFF")
        .build_target("bungee_library")
        .build();

    // link
    println!(
        "cargo:rustc-link-search=native={}/{}",
        out_dir.join("build").display(),
        profile
    );
    println!(
        "cargo:rustc-link-search=native={}/submodules/kissfft/{}",
        out_dir.join("build").display(),
        profile
    );

    println!("cargo:rustc-link-lib=static=bungee");
    println!("cargo:rustc-link-lib=static=kissfft-float");
}
