# Changelog

## [0.1.2] - 2026-07-31

### Added

- Allowed orbiting the 3D character by dragging empty viewport space while keeping character-origin
  drags available for painting.

### Fixed

- Prevented macOS Open and Save dialogs from crashing when files are dragged into them.
- Preserved dirty-document confirmations and deferred New, Open, and Quit actions when asynchronous
  saves are canceled or fail.

## [0.1.1] - 2026-07-31

### Fixed

- Made colors on the 3D model match the palette and active-color controls across gamma and sRGB
  framebuffers, including accurate midtones and saturated colors while preserving transparency and
  outer-layer blending.
