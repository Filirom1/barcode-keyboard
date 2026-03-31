# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

**This project runs on NixOS. All build commands must be run inside nix-shell.**

### WASM Build (critical: requires unwrapped clang for `ring`)

The `ring` crypto crate compiles C code for WASM. The nix-wrapped clang adds `-fzero-call-used-regs=used-gpr` which is unsupported for WASM targets — use the unwrapped binary directly.

```bash
# Debug build
nix-shell -p cargo llvmPackages.lld llvmPackages.llvm binaryen --run "
  export CC_wasm32_unknown_unknown=/nix/store/zq0z7g2s1jnks1995nmp8ii6lp0xh6wq-clang-21.1.7/bin/clang
  export AR_wasm32_unknown_unknown=llvm-ar
  cargo build --target=wasm32-unknown-unknown
  PATH=/home/nixos/.cargo/bin:\$PATH wasm-bindgen ./target/wasm32-unknown-unknown/debug/barcode_keyboard.wasm --out-dir=public/wasm --weak-refs --target=web --debug
"

# Release build (for deployment)
nix-shell -p cargo llvmPackages.lld llvmPackages.llvm binaryen --run "
  export CC_wasm32_unknown_unknown=/nix/store/zq0z7g2s1jnks1995nmp8ii6lp0xh6wq-clang-21.1.7/bin/clang
  export AR_wasm32_unknown_unknown=llvm-ar
  cargo build --target=wasm32-unknown-unknown --release
  PATH=/home/nixos/.cargo/bin:\$PATH wasm-bindgen ./target/wasm32-unknown-unknown/release/barcode_keyboard.wasm --out-dir=public/wasm --weak-refs --target=web
  wasm-opt --enable-nontrapping-float-to-int --enable-bulk-memory -Os -o public/wasm/barcode_keyboard_bg.wasm public/wasm/barcode_keyboard_bg.wasm
"
```

`wasm-bindgen-cli` must be version **0.2.114** (matching Cargo.toml pin). Install once with:
```bash
cargo install wasm-bindgen-cli --version 0.2.114 --locked
```

Sanity check after build — must be zero:
```bash
grep -c 'from "env"' public/wasm/barcode_keyboard.js
```

### CLI Build & Run

```bash
nix-shell -p cargo --run "cargo run --features cli -- accept"
nix-shell -p cargo --run "cargo run --features cli -- connect <ENDPOINT_ID> 'hello'"
```

### Desktop Keyboard App Build & Run

Requires `xdotool` (provides `libxdo`) on Linux for enigo keyboard simulation.

```bash
# Debug build
nix-shell -p cargo xdotool --run "cargo build --features keyboard --bin keyboard"

# Run
nix-shell -p cargo xdotool --run "cargo run --features keyboard --bin keyboard"
```

### Dev Server

```bash
nix-shell -p nodePackages.http-server --run "http-server --cors -a localhost public/"
```

## Architecture

The codebase has three compilation targets sharing a common protocol core:

```
src/node.rs      ← platform-agnostic protocol logic (EchoNode, Echo handler, event types)
src/wasm.rs      ← thin WASM wrapper (type conversions, JS ReadableStream, logging setup)
src/bin/cli.rs   ← thin CLI wrapper
public/          ← HTML/JS frontend consuming the WASM
```

### Protocol Layer (`src/node.rs`)

`EchoNode` owns an iroh `Router` and a broadcast channel for accept events. The `Echo` struct implements iroh's `ProtocolHandler` trait with ALPN `b"iroh/example-browser-echo/0"`.

**Accept side**: reads all bytes from incoming bi-directional QUIC stream via `recv.read_to_end(1_000_000)` (quinn-native API, not `AsyncReadExt::read_to_end` — they conflict), echoes back, emits `AcceptEvent::Received { endpoint_id, content }`.

**Connect side**: spawns a task that opens a bi-directional stream, sends the payload, and drains the echo response. Events are sent over an `async_channel`.

Both `accept_events()` and `connect()` return `BoxStream` / `impl Stream` consumed by the WASM or CLI layers.

**Event types** (serialized with `#[serde(tag = "type", rename_all = "camelCase")]`):
- `AcceptEvent`: `accepted`, `received { endpointId, content }`, `closed { endpointId, error }`
- `ConnectEvent`: `connected`, `sent { bytesSent }`, `received { bytesReceived }`, `closed { error }`

### WASM Wrapper (`src/wasm.rs`)

`EchoNode` in wasm.rs wraps `node::EchoNode`. The key bridge: `into_js_readable_stream()` takes any `Stream<Item: Serialize>`, serializes each item via `serde_wasm_bindgen`, and wraps it in a `wasm_streams::ReadableStream` — making it a native JS async iterable.

The `start()` function (called automatically via `#[wasm_bindgen(start)]`) configures logging to redirect `TRACE` events to `DEBUG` level on the browser console.

### Frontend Pages (`public/`)

- **`index.html` + `main.js`**: Original echo demo. URL params `?connect=<id>&payload=<msg>` auto-fill and auto-submit the form.
- **`receiver.html`**: Barcode receiver (PC side). Spawns iroh node, renders a QR code (via local `qr-creator.min.js`) pointing to `scanner.html?endpoint=<id>`, listens for `received` events, auto-copies content to clipboard.
- **`scanner.html`**: Barcode scanner (phone side). Camera via `getUserMedia`, decoding via `@undecaf/zbar-wasm` (CDN), sends each detected barcode by calling `node.connect(pcEndpoint, code)` and consuming the returned stream.

The scanner page reads `?endpoint=<id>` from the URL to know where to send barcodes. Deduplication: same code within 2 seconds is ignored.

## Key Constraints

- `wasm-bindgen` version is pinned to `=0.2.114` — the CLI tool version must match exactly.
- `getrandom` is configured via `.cargo/config.toml` rustflag `--cfg getrandom_backend="wasm_js"` (not the crate feature) for the wasm32 target.
- When modifying `node.rs` for WASM: use quinn-native `recv.read_to_end(max_bytes)` and `send.write_all()` with `.map_err(|e| std::io::Error::other(e))?` — the Iroh stream types have inherent methods that shadow `AsyncReadExt`/`AsyncWriteExt` and return incompatible error types.
- The `public/wasm/` directory is generated — never edit those files manually. Re-run the build pipeline after any Rust changes.
