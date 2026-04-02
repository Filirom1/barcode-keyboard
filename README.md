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
| `public/scanner.html` | Browser-based scanner (phone side): camera viewfinder with zbar-wasm decoding; picks the largest detected barcode when multiple are in frame; recovers the camera stream automatically after the screen wakes up |
| `src/bin/keyboard.rs` | Native desktop app: types each scan as keystrokes (keyboard wedge) via `xdotool` / PowerShell SendKeys; includes a full Preferences panel |

## Quick start — web version

The web app is automatically deployed to GitHub Pages on every release:
**https://filirom1.github.io/barcode-keyboard/**

1. Open the URL above on your PC — you get a QR code
2. Scan the QR code with your phone
3. Scan product barcodes — they appear on the PC and are copied to clipboard

> Requires a browser with WebRTC support (Chrome, Firefox, Safari 16+). The phone scanner uses `getUserMedia` and `@undecaf/zbar-wasm`; no app install needed.

## Quick start — desktop keyboard app

Download the pre-built binary for your platform from the [Releases](../../releases) page.

**Linux / WSL2:**
```sh
./keyboard --terminal https://filirom1.github.io/barcode-keyboard
```

**Windows:**
```
keyboard.exe --terminal https://filirom1.github.io/barcode-keyboard
```

The app prints a QR code in the terminal. Scan it with your phone, then scan any barcode — it gets typed into whatever window is focused on your PC, followed by Enter.

On WSL2, keystrokes are injected via `powershell.exe SendKeys` (types into focused Windows windows). On native Linux, `xdotool` is used.

> A GUI mode is also available (run without `--terminal`), but requires a working display server.

## Preferences

The desktop app has a **Preferences** panel (collapsible, below the QR code) for customising scanner behaviour. Settings are persisted to `~/.config/barcode-keyboard/config.json`.

### Barcode formats

Choose which barcode types the phone scanner recognises:

| Format | Default |
|---|---|
| EAN-13 | enabled |
| EAN-8 | enabled |
| UPC-A | enabled |
| Code 39 | enabled |
| Code 128 | enabled |
| UPC-E | disabled |
| QR Code | disabled |
| PDF417 | disabled |
| ITF | disabled |
| Codabar | disabled |
| Code 93 | disabled |
| DataBar | disabled |

When you change the format selection, the QR code regenerates automatically — the phone picks up the new configuration just by rescanning it. A `?formats=` parameter is appended to the scanner URL only when the selection differs from the defaults, keeping the QR code as small as possible.

### Key after scan (suffix)

| Option | Behaviour |
|---|---|
| Enter (default) | Press Return after typing |
| Tab | Press Tab — useful for multi-field forms |
| None | Type the barcode only |

### Deduplication

Controls whether repeated scans of the same barcode are ignored on the PC side (the phone side always applies a 5-second window).

**Timeout** — window during which a duplicate is suppressed:

| Option | Value |
|---|---|
| Disabled | 0 s |
| 2 s | 2 |
| 5 s | 5 |
| 10 s (default) | 10 |
| 30 s | 30 |
| 60 s | 60 |

**Mode** — what counts as a duplicate:

| Option | Behaviour |
|---|---|
| Consecutive (default) | Ignore if same as the immediately previous scan |
| Any | Ignore if the same code was seen at any point in the session |
| Disabled | Accept all scans unconditionally |

### Prefix

Prepend a fixed string before every typed barcode (e.g. `ITEM:`).

### Transform

| Option | Behaviour |
|---|---|
| None (default) | Type as-is |
| Uppercase | Convert to uppercase |
| Lowercase | Convert to lowercase |
| Trim | Strip leading/trailing whitespace |

### Ignore pattern (regex)

Drop barcodes matching a regular expression before they are typed. Leave blank to disable.

### Copy only (no keystroke)

When enabled, the barcode is copied to the clipboard instead of injected as keystrokes — useful when the target application is not focused.

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

## Built with

- [iroh](https://iroh.computer) — direct peer-to-peer connectivity over QUIC/WebRTC; no server needed
- [zbar-wasm](https://github.com/undecaf/zbar-wasm) — barcode decoding in the browser via WebAssembly
- [egui / eframe](https://github.com/emilk/egui) — immediate-mode GUI for the native desktop app

## License

MIT
