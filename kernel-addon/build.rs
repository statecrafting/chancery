// SPDX-License-Identifier: Apache-2.0

fn main() {
    // The napi cdylib linker setup is only needed when the addon surface is
    // built. When the crate is compiled with `--no-default-features` (pure Rust
    // core, e.g. host-only tests), skip it so there is no napi coupling at all.
    if std::env::var("CARGO_FEATURE_NAPI").is_ok() {
        napi_build::setup();
    }
}
