# Preferences — TODO

Formats are implemented. Below are the remaining preferences to add, grouped by where they are applied.

---

## Desktop app (processed after barcode arrives on PC)

All stored in `~/.config/barcode-keyboard/config.json`.

### Suffix after scan
Append a key after each typed barcode. Inspired by BinaryEye's separator setting.

| Option | Behaviour |
|---|---|
| Enter (default) | Press Return after typing |
| Tab | Press Tab after typing — useful for multi-field forms |
| None | Type the barcode only |

Implementation: add `suffix` field to `Config`, update `type_xdotool` / `type_powershell` / `type_enigo` helpers, add a `ComboBox` in the Preferences panel.

---

### Deduplication
Currently hardcoded: 10 s on PC side, 5 s on phone side.

**Timeout** — make the window configurable:

| Option | Value |
|---|---|
| Disabled | 0 s |
| 2 s | 2 |
| 5 s | 5 |
| 10 s (default) | 10 |
| 30 s | 30 |
| 60 s | 60 |

**Mode** — choose what counts as a duplicate:

| Option | Behaviour |
|---|---|
| Consecutive (default) | Ignore if same as the immediately previous scan |
| Any | Ignore if the same code was seen at any point in the session |
| Disabled | Accept all scans unconditionally |

Implementation: add `dedup_secs` and `dedup_mode` fields to `Config`, add `ComboBox` widgets in the Preferences panel, apply in both GUI and terminal modes.

---

### Prefix
Prepend a fixed string before every typed barcode (e.g. `ITEM:`).

Implementation: add `prefix: String` field to `Config`, add a `TextEdit` in the Preferences panel, prepend when injecting keystrokes.

---

### Transform
Apply a transformation to the barcode content before typing.

| Option | Behaviour |
|---|---|
| None (default) | Type as-is |
| Uppercase | Convert to uppercase |
| Lowercase | Convert to lowercase |
| Trim | Strip leading/trailing whitespace |

Implementation: add `transform` field to `Config`, add a `ComboBox` in the Preferences panel, apply before keystroke injection.

---

### Ignore codes (regex filter)
Drop barcodes matching a regex pattern before they are typed (inspired by BinaryEye's ignore list).

Implementation: add `ignore_pattern: Option<String>` to `Config`, add a `TextEdit` in the Preferences panel, test pattern against each incoming barcode.

---

### Copy only (no keystroke)
When enabled, copy the barcode to the clipboard instead of injecting keystrokes — useful when the target app is not focused.

Implementation: add `copy_only: bool` to `Config`, add a `Checkbox` in the Preferences panel, skip `type_*` calls when set.

---

## Phone side via QR code URL parameter

These must be configured on the phone (scanner.html cannot receive them after the fact). The desktop app encodes them into the scanner URL; rescanning the QR code picks up changes automatically.

### Vibrate (`?vibrate=0`)
Disable haptic feedback on scan. Default: enabled.

Implementation: add `vibrate: bool` to `Config`; append `&vibrate=0` to scanner URL when disabled; read param in `scanner.html` and gate `navigator.vibrate()` on it.

### Camera (`?camera=front`)
Select front or rear camera. Default: rear (`environment`).

Implementation: add `camera: String` (`"rear"` / `"front"`) to `Config`; append `&camera=front` to scanner URL; read param in `scanner.html` and pass to `getUserMedia` `facingMode`.

### Torch (`?torch=1`)
Start with the flashlight on. Default: off.

Implementation: add `torch: bool` to `Config`; append `&torch=1` to scanner URL; read param in `scanner.html` and apply via `ImageCapture` / `MediaStreamTrack.applyConstraints`.
