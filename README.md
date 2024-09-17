
### Nix setup for intellij

The toolchain path is where cargo is listed after running `which -a cargo`. You have to pick the correct version based on the toolchain for the project.

The std-lib needs to be downloaded with `rustup -v component add rust-src` and then lives in the same path as cargo like `/home/nhyne/.rustup/toolchains/nightly-2024-09-17-x86_64-unknown-linux-gnu/lib/rustlib/src/rust`.
