# Tilda MkV STM32 Firmware

This is the prototype firmware in Rust for the secondary STM32 processor on the TiLDA MkV badge for the cancelled Electromagnetic Field 2020 event.

We planned to use an STM32F0x2 device to act as a custom USB interface to allow the main ESP32 processor to be accessed either over a conventional CDC USB serial interface, or over a WebUSB endpoint from a custom web-based IDE.

It's likely that Web Serial would have removed the need for this (at the expense of a tiny bit of user experience), however Web Serial was not scheduled to be released in Chrome by the time of the event.

## How do I embedded Rust

Best check out [the book](https://rust-embedded.github.io/book/) first.

* [Install Rust](https://rustup.rs/) (or `rustup update` to make sure you're running the latest stable Rust).
* `rustup target add thumbv6m-none-eabi` to install the ARM toolchain.
* `cargo install cargo-binutils`
* `rustup component add llvm-tools-preview`

## Building/debugging

You need some kind of in-circuit debugger. This repo is set up for debugging with a Black Magic Probe (if you're
using one, you'll need to change the device name in `bmp.gdb` to the gdb (first) interface of your device.

If you want to use another debug probe, you'll need to change the `runner = ` line in `.cargo/config` to point
to an appropriate gdb file.

Once that's set up, you should be able to:

`cargo run --release`

Which will hopefully compile, load the code, and drop you at a gdb breakpoint at the beginning of the
main function (type `c` to continue).

(You need to build in release mode - there isn't enough flash space for a debug build.)

## Semihosting debugging

You can use the [cortex-m-semihosting](https://docs.rs/cortex-m-semihosting) crate to print debugging
output using the `hprintln!` and `dbg!` macros.
