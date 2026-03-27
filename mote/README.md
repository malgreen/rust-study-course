# Mote

(based on `esp-rs no_std` training repositories)

## Prerequisites

This package requires a Rust setup that supports ESP32.

The installation depends on the architecture of the target ESP32.

1. (**xtensa**):
    1. `cargo install espup --locked`
    1. `espup install`
1. (**risc-v**)
    1. follow [this guide](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html#risc-v-devices)
1. `cargo install espflash --locked`
