# KSM Regulator
_KSM regulator is a daemon to automatically manage KSM_

# Building with Cargo

Building with Cargo is supported with `cargo build --release`

# Building with Xargo

To build with Xargo (which will allow to do LTO optimization with stdlib, resulting 30% less executable size)

Use this command line:
`xargo build --release --target x86_64-unknown-linux-gnu`
