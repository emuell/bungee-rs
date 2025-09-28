# Low-level Rust FFI Bindings for Bungee Audio Time-Stretching Library

`bungee-sys` provides low-level, unsafe Rust bindings for the [Bungee](https://github.com/bungee-audio-stretch/bungee) C++ audio time-stretching and pitch-shifting library.

See also [`bungee-rs`](../README.md) which uses this crate to provide somewhat *safer* and more high-level bindings.

Note: When building this crate locally, clone the repository with `git clone --recurse-submodules <url>`. It contains the bungee C++ source code as git submodule.

### Prerequisites

- Rust toolchain (stable)
- C++17 compatible compiler (msvc, clang++ or g++)
- CMake (for building Bungee dependencies)

## License

`bungee-sys` is licensed under the MPL-2.0 license, consistent with the upstream Bungee C++ library.
