# TODO

## Phone scanner (scanner.html)

### Resizable viewfinder

Allow the user to resize the scan rectangle by pinching with two fingers (pinch-to-zoom gesture), so they can adapt the viewfinder to narrow or large barcodes without changing the camera zoom.

Implementation notes:
- Listen for `touchstart`, `touchmove`, `touchend` on the `#viewfinder` element (or on `#wrap` and hit-test for the viewfinder).
- On two-finger touch, track the distance between the two touch points. Compare to the initial distance on `touchstart` to derive a scale factor.
- Apply the scale factor to both the `width` and `height` CSS custom properties (or inline styles) of `#viewfinder`, clamped to a min/max range (e.g. `min(30vw, 120px)` … `min(95vw, 500px)` wide).
- Persist the chosen size in `localStorage` (`viewfinderWidth`, `viewfinderHeight`) so it survives page reloads.
- Restore persisted size on page load before the first frame.
