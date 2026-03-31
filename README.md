# Barcode Keyboard

Turn your smartphone into a wireless barcode scanner that types into any PC application — no drivers, no pairing, no server.

Built with [iroh](https://iroh.computer) for direct peer-to-peer connectivity over WebRTC/QUIC. The phone and PC connect directly; scanned barcodes are sent over an encrypted P2P channel and injected as keystrokes on the PC.

## How it works

```
Phone camera  →  scanner.html  ──iroh P2P──►  receiver.html  →  clipboard
                                          └──►  keyboard app   →  keystroke injection
```

1. **PC**: open `receiver.html` in a browser, or launch the native desktop app — you get a QR code
2. **Phone**: scan the QR code to open `scanner.html` — point at any barcode
3. Each scan is sent over iroh and typed into whatever window is focused on the PC

No account, no cloud, no pairing ritual. The QR code encodes the iroh endpoint ID; that's all the phone needs to connect directly.

## Components

| Component | Description |
|-----------|-------------|
| `public/receiver.html` | Browser-based receiver (PC side): shows QR code, lists received barcodes, auto-copies to clipboard |
| `public/scanner.html` | Browser-based scanner (phone side): camera viewfinder with zbar-wasm decoding |
| `src/bin/keyboard.rs` | Native desktop app: types each scan as keystrokes (keyboard wedge) via `xdotool` / PowerShell SendKeys |

## Quick start — web version

Deploy the `public/` directory to any static host. On [Netlify Drop](https://app.netlify.com/drop) it takes 30 seconds.

Then:
1. Open `https://your-site.netlify.app/receiver.html` on your PC
2. Scan the QR code with your phone
3. Scan product barcodes — they appear on the PC and are copied to clipboard

> The web receiver requires a browser with WebRTC support (Chrome, Firefox, Safari 16+). The phone scanner uses `getUserMedia` and `@undecaf/zbar-wasm`; no app install needed.

## Quick start — desktop keyboard app

Download the pre-built binary for your platform from the [Releases](../../releases) page.

**Linux / WSL2:**
```sh
./keyboard --terminal https://your-site.netlify.app
```

**Windows:**
```
keyboard.exe --terminal https://your-site.netlify.app
```

The app prints a QR code in the terminal. Scan it with your phone, then scan any barcode — it gets typed into whatever window is focused on your PC, followed by Enter.

On WSL2, keystrokes are injected via `powershell.exe SendKeys` (types into focused Windows windows). On native Linux, `xdotool` is used.

> A GUI mode is also available (run without `--terminal`), but requires a working display server.

## Deduplication

Same barcode scanned within **10 seconds** is silently ignored on the PC side (phone side: 5 seconds). This prevents the camera from sending the same barcode frame-by-frame. To re-scan the same barcode intentionally, wait 10 seconds or restart the app.

## Building

### Prerequisites

- Rust stable (`rustup`)
- wasm-bindgen-cli 0.2.114 (must match the pinned version):
  ```sh
  cargo install wasm-bindgen-cli --version 0.2.114 --locked
  ```

### WASM (web frontend)

```sh
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen ./target/wasm32-unknown-unknown/release/barcode_keyboard.wasm \
  --out-dir=public/wasm --weak-refs --target=web
# Optional: optimise size
wasm-opt --enable-nontrapping-float-to-int --enable-bulk-memory \
  -Os -o public/wasm/barcode_keyboard_bg.wasm public/wasm/barcode_keyboard_bg.wasm
```

> **NixOS only**: the nix-wrapped clang adds `-fzero-call-used-regs=used-gpr` which breaks ring's C compilation for WASM. Use the unwrapped binary:
> ```sh
> nix-shell -p cargo llvmPackages.lld llvmPackages.llvm binaryen --run "
>   export CC_wasm32_unknown_unknown=/nix/store/.../clang-21.1.7/bin/clang
>   export AR_wasm32_unknown_unknown=llvm-ar
>   cargo build --target=wasm32-unknown-unknown --release
>   ...
> "
> ```
> See `CLAUDE.md` for the exact command.

Sanity check — must print `0`:
```sh
grep -c 'from "env"' public/wasm/barcode_keyboard.js
```

### Desktop app — Linux

```sh
# Debian/Ubuntu
sudo apt-get install -y libx11-dev libxcursor-dev libxrandr-dev libxi-dev libgl1-mesa-dev xdotool

cargo build --features keyboard --bin keyboard --release
```

### Desktop app — Windows

```sh
cargo build --features keyboard --bin keyboard --release
```

No extra dependencies needed on Windows; enigo uses the Win32 API directly.

### Dev server

```sh
npx http-server --cors -a localhost public/
# or with nix:
nix-shell -p nodePackages.http-server --run "http-server --cors -a localhost public/"
```

## Architecture

```
src/node.rs     platform-agnostic protocol (EchoNode, Echo handler, event types)
src/wasm.rs     WASM wrapper (type conversions, JS ReadableStream bridge)
src/bin/
  keyboard.rs   desktop app (egui GUI + terminal mode)
  cli.rs        debug CLI (accept/connect)
public/
  receiver.html PC-side browser app
  scanner.html  phone-side browser app
  wasm/         generated — do not edit
```

The iroh protocol uses ALPN `b"iroh/example-browser-echo/0"`. Both sides speak the same protocol: connect → send barcode bytes → receive echo → done.

## License

Apache-2.0 OR MIT
