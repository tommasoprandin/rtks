## Prerequisites
1. Rustup (https://www.rust-lang.org/tools/install)
2. An IDE (VS Code with Rust Extension, RustRover, Neovim, ...)
3. QEMU (see https://www.qemu.org/download/)
4. `defmt-print` (install with `cargo install defmt-print`)
5. `flip-link` (install with `cargo install flip-link`)

## Getting Started
To compile the project with debug config:
```
cargo build
```
or, with optimized config:
```
cargo build --release
```

To run the project on a QEMU emulated machine:
```
cargo run
```
or:
```
cargo run --release
```

The runner is set up to launch a QEMU instance that prints to the host via semihosting, `defmt-print` will decode defmt logs and print human-readable logs.

## Crates and setup

The example is based on the crates `cortex-m` and `cortex-m-rt` which provide runtime initialization (vector table, .bss and .data section, stack pointer, etc...), and other useful stuff (eg entrypoint macro and critical section implementation).

`defmt` is used for logging, it allows for very efficient data transfer and it lets us use the same code between local QEMU testing and actual hardware (just need to change the global logger).

`memory.x` is a super basic linker script, just enough to make this basic example boot and work. In order to protect from stack overflow undefined behaviour `flip-link` linker is used.

Currently the example is set to compile and run on a Cortex-M4 microprocessor, the machine type is netduinoplus2 (since it is implemented in QEMU). The relevant documents (datasheet, reference manual and programming manual) are in the `datasheets` folder.
