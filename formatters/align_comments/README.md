# align_comments

A WASM component for pruner that aligns Clojure comments using tree-sitter.

## Building

This project compiles to a WASM component targeting `wasm32-wasip2`. Because it uses tree-sitter (which has C code), you
need a C compiler and WASI sysroot that can target WebAssembly.

### Requirements

- Rust with the `wasm32-wasip2` target
- A clang/LLVM toolchain with WASM support
- WASI libc sysroot

### macOS (Homebrew)

```bash
# Install dependencies
brew install llvm wasi-libc
rustup target add wasm32-wasip2

# Build
WASI_SYSROOT=/opt/homebrew/Cellar/wasi-libc/29/share/wasi-sysroot \
CC=/opt/homebrew/opt/llvm/bin/clang \
just build
```

Note: The wasi-libc version number (29) may differ. Check with `brew info wasi-libc`.

### Linux

Install WASI SDK from https://github.com/WebAssembly/wasi-sdk/releases and set:

```bash
WASI_SYSROOT=/path/to/wasi-sdk/share/wasi-sysroot \
CC=/path/to/wasi-sdk/bin/clang \
just build
```

### Output

The compiled component will be at:

```
dist/align_comments.wasm
```
