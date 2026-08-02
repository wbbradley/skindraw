# Changelog

## [0.1.3] - 2026-08-01

### Added

- Added an always-visible Front, Back, Left, or Right orientation badge that follows the camera.
- Added an Exploded body-part layout so normally occluded interior faces can be painted while all
  cuboids remain visible and interactive.
- Added independently sampled Gaussian hue, saturation, and value jitter for every brush and fill
  texel, with alpha preserved and settings retained across launches.

### Changed

- Exploded layout widens the camera framing, is mutually exclusive with Solo mode, and returns to
  Joined layout when Escape is pressed.

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
