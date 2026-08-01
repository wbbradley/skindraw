# Completed

## Build the UI-independent skin editing core

Implemented a UI- and GPU-independent Rust library for Minecraft skin editing. It includes typed
Classic/Slim model geometry and official atlas mappings, orthographic camera rays and layer-aware
box picking, a 64×64 RGBA skin and PNG codec, face-clipped interpolated brushes, stroke-level
history, saved-baseline dirty tracking, and path-based document load/save. Added comprehensive
module tests covering atlas tables, picking, camera limits, brush behavior, history, alpha rules,
PNG formats, errors, and exact round trips.

### Original task

## Build the UI-independent skin editing core

Create the CPU-side foundation for a native Minecraft Java Edition skin editor so later UI and
renderer work can consume tested domain APIs without depending on `egui` or `wgpu`.

### Model, atlas, and picking

- Define typed model concepts for Classic and Slim models, body parts, base and outer layers, faces,
  texels, atlas rectangles, rays, and hit records.
- Generate the head, torso, separate left and right arms, and separate left and right legs as boxes
  in model-pixel coordinates. Use +Y as up and +Z as the character's front. Use four-pixel Classic
  arms and three-pixel Slim arms.
- Inflate the head outer layer by 0.5 model pixels and jacket, sleeve, and trouser layers by 0.25
  model pixels.
- Encode the official 64×64 Java atlas rectangles for every part, face, and layer, including Slim
  arm widths. Map face-local coordinates to exact texels with explicit orientation and flip
  metadata, and expose each face rectangle for brush clipping.
- Implement an orthographic orbit camera with unrestricted yaw, pitch clamped just short of ±90°,
  and no roll. Generate pointer rays from viewport coordinates without UI toolkit types.
- Intersect enabled model boxes nearest-first and return part, layer, face, distance, and atlas
  texel. Account for model kind and face orientation; allow outer layers to win when nearer and
  filter disabled layers.

### Skin files, editing, and history

- Store one 64×64 `[u8; 4]` CPU pixel buffer as the source of truth. Generate visible blank Classic
  and Slim skins with opaque white base islands and transparent outer and unused pixels. Switching
  arm model is presentation state and must not alter pixels or dirty state.
- Decode standard 64×64 PNG color formats into 8-bit RGBA, reject other dimensions clearly, and
  encode lossless 64×64 RGBA PNGs. Add path-based load and save wrappers.
- Implement brush sizes 1, 2, and 4, RGBA replacement including transparent colors, integer-line
  interpolation, and clipping to the currently hit face. Crossing to a different atlas face starts
  a new interpolation segment.
- Aggregate repeated changes to the same pixel into one stroke. Treat one press/drag gesture as one
  undo entry, truncate redo after a branched edit, and restore clean state when undo returns pixels
  to the saved baseline.
- Add document state for the current path, saved baseline, history, undo and redo, dirty detection,
  and marking a successful save.

### Public interfaces

- Export `ModelKind`, `BodyPart`, `Layer`, `Face`, `Texel`, `FaceRegion`, `Ray`, and `ModelHit`.
- Export `model_boxes(kind)`, `face_region(kind, part, layer, face)`, and face-local-to-texel
  mapping.
- Export camera pointer-ray generation, picking, and pitch/yaw orbit updates.
- Export `Skin`, `SkinDocument`, `StrokeBuilder`, `BrushSize`, undo, redo, dirty state, save
  baselines, and PNG load/save.
- Keep `src/main.rs` as the existing binary stub in this checkpoint and expose the domain modules
  through `src/lib.rs`.

### Tests and completion checks

- Add independent table-driven expectations for every face of every part and layer in Classic and
  Slim modes, including rectangle sizes, known corners, orientation, and mirrored faces.
- Test front, back, left, right, top, and bottom ray hits, misses, Slim arm bounds, nearest outer
  layer selection, disabled-layer filtering, and camera pitch bounds.
- Test 1-, 2-, and 4-pixel brush clipping, fast-drag interpolation, face-boundary discontinuity,
  transparent edits, stroke grouping, duplicate-pixel aggregation, undo/redo, redo truncation, and
  saved-baseline dirty state.
- Test blank-skin alpha rules, RGB/RGBA decoding, invalid-size rejection, malformed PNG errors,
  filesystem load/save, and byte-accurate RGBA round trips.
- Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo nextest run`.

## Integrate the native renderer and model-view input

Replaced the binary stub with a native `eframe` WGPU application. The focused app and renderer
modules now draw Classic or Slim base and outer meshes through an `egui_wgpu` callback using a
shared nearest-filtered skin texture, an offscreen color target, and a depth buffer. Painting uses
the tested CPU ray picker and stroke history, uploads only changed texture bounds, and includes
face highlighting, view-scoped Space-drag orbit, visibility toggles, three brush sizes, a palette,
and custom RGBA controls. Added renderer-adjacent tests and verified the running macOS app's
painting, brushes, history, orbit, layer controls, and model switching; the final release phase
retains the full Linux and macOS acceptance pass.

### Original task

## Integrate the native renderer and model-view input

Replace the binary stub with a native macOS and Linux `eframe`/`egui` app shell and render the model
through the `wgpu` renderer exposed by `eframe` using an `egui_wgpu` paint callback. Do not use Bevy:
the six-part model and tested direct CPU picking do not need its game loop or ECS.

- Render the tested generated model with one shared GPU skin texture, nearest-neighbor sampling, a
  depth buffer, and both base and slightly inflated outer geometry.
- Upload changed pixel regions with `wgpu::Queue::write_texture` so edits appear immediately without
  rebuilding the model.
- Keep an orthographic camera aimed at the model. Paint with primary-button press/drag; while Space
  is held, use primary drag to orbit yaw and pitch. Keep all input scoped to the 3D view.
- Connect CPU picking to posed model boxes and exact atlas texels. Add a small cursor or face
  highlight for valid hits.
- Add base/outer visibility toggles so pixels hidden by an outer layer remain reachable.
- Put a fixed palette, active color, RGBA custom color editor, and 1/2/4 brush controls in a right
  sidebar. Let the user switch Classic/Slim presentation without modifying pixels.
- Keep native startup, app state, renderer, and paint callback in focused modules; retain the CPU
  core's independence from `egui` and GPU types.
- Test renderer-adjacent state where practical, run formatter/tests/Clippy, and manually check
  painting every visible face and brush size, Space-drag orbit, layer toggles, undo/redo, and arm
  switching on Linux and macOS.

## Complete the desktop file workflow and platform release

Completed the native document lifecycle with New Classic/Slim, Open, Save, and Save As controls;
native PNG dialogs; current-file and dirty-state indicators; in-app file and validation errors; and
cross-platform shortcuts for document commands and history. A shared guarded-action flow now
protects dirty documents before New, Open, and native Quit, with Save, Discard, and Cancel choices.
Added tests for guarded replacement, failed open/save preservation, shortcut coverage, filename
normalization, and exact RGBA save/reopen behavior. Formatter, warnings-denied Clippy, and all 30
tests pass, and the macOS debug app was checked for native dialogs, shortcuts, dirty prompts,
painting, camera orbit, both layers, history, arm switching, and clean shutdown. Linux testing was
explicitly waived by the user.

### Original task

## Complete the desktop file workflow and platform release

Finish the native editor's document lifecycle, dialogs, shortcuts, validation UI, and cross-platform
acceptance checks.

- Start with a valid blank or bundled skin and support New Classic/Slim, Open, Save, and Save As for
  lossless 64×64 Java Edition PNGs. Keep legacy 64×32 conversion and Bedrock packs out of scope.
- Use `rfd` or another small native dialog crate for Open and Save As. Surface file and validation
  failures in the app rather than only on stderr.
- Track unsaved edits and ask before New, Open, or Quit discards them.
- Add common desktop shortcuts for New, Open, Save, Save As, Undo, and Redo.
- Keep `src/main.rs` limited to native startup and add only the platform dependencies needed by the
  app.
- Run formatter, tests, and Clippy with warnings denied.
- Manually check debug builds on Linux and macOS: create and open a skin, paint all visible faces,
  rotate, edit both layers, undo and redo, switch arm type, save, reopen, and confirm pixels and alpha
  are unchanged.
- Complete the editor when it starts on both targets without panic, all controls work, painting
  selects the texel under the pointer from several camera angles, and a saved PNG is accepted as a
  Java Edition skin.

## Preview the exact brush footprint

Replaced the face-wide hover tint with transparent-safe GPU overlay geometry for the exact clipped
1×1, 2×2, or 4×4 brush footprint. Painting and preview now share one CPU footprint calculation,
including the established even-size anchor and face-edge clipping. Added tests for footprint
anchoring, clipping, overlay geometry, and flipped UV orientation, and visually confirmed that the
4×4 preview exactly matches the resulting paint on a transparent outer layer.

### Original task

## Preview the exact brush footprint

Replace the current whole-face hover tint with a preview of the exact texels that the selected
brush would paint.

- Extract a shared brush-footprint calculation from `src/brush.rs` and use it for both painting and
  preview generation so 1×1, 2×2, and 4×4 anchoring cannot drift apart. Preserve the current
  even-sized brush anchoring and clip the footprint to the hit face's `AtlasRect`.
- Pass the clipped footprint through `ModelPaintCallback` and render a clear translucent overlay or
  outline for each covered texel in `src/renderer.rs`. Respect face UV flips and model kind, avoid
  depth fighting and atlas bleeding, and keep the preview visible over transparent outer-layer
  texels.
- Remove the per-face `highlight` vertex and shader behavior. Show no preview without a valid hit,
  and suppress it while an orbit drag is active.
- Add table-driven tests for all brush sizes, face-edge clipping, and flipped faces, plus
  renderer-adjacent tests proving only footprint texels receive preview geometry.
- Manually verify that the preview matches the resulting stroke on opaque and transparent faces
  from several camera angles, then run formatting, warnings-denied Clippy, and the full test suite.

## Orbit with Shift-drag

Replaced Space-drag orbit with Shift-drag and centralized model-view gesture classification so
modified drags orbit, plain drags paint, and input outside the view does neither. Entering orbit
commits any active stroke to prevent interpolation across camera movement. Updated help and cursor
behavior, added modifier-transition tests, and verified the native Shift-drag rotates the model.

### Original task

## Orbit with Shift-drag

Replace the model view's Space + primary-button drag orbit gesture with Shift + primary-button
drag.

- Update input classification in `src/app.rs` so Shift + primary drag orbits and never paints,
  while an unmodified primary drag continues to paint. Keep both interactions scoped to the 3D
  view.
- Finish any active paint stroke when the gesture changes into orbit mode so pressing or releasing
  Shift during one pointer press cannot create an interpolated line across an orbit.
- Preserve the existing yaw and corrected pitch directions, camera bounds, and grab/crosshair
  cursor feedback.
- Update the sidebar help text and its content-width measurement from Space to Shift.
- Factor gesture classification into testable logic and cover plain paint, Shift-orbit, modifier
  transitions, and pointer activity outside the view. Run formatting, warnings-denied Clippy, and
  the full test suite.

## Solo a body part for hidden-face editing

Added Ctrl-click body-part soloing with Escape exit, document-independent state, selected-part-only
solid rendering and picking, and faint untextured edge guides for the other five body parts. Both
base and outer layers remain governed by the existing visibility toggles, Shift-drag orbit remains
available, and filtered picking reaches normally occluded interior faces. Added input, state,
picking, geometry-filter, and guide-generation tests and visually verified solo entry, guide
rendering, Escape exit, orbit, and an inner arm-face hit.

### Original task

## Solo a body part for hidden-face editing

Add a DAW-style solo mode that exposes otherwise occluded faces of one body part.

- Track `Option<BodyPart>` presentation state in `SkinDrawApp`. Ctrl + primary click on a valid
  model hit enters solo mode for that whole body part and consumes the click without painting;
  Escape exits solo mode. Solo state must not modify skin pixels, dirty state, or history.
- While soloed, render and pick only the selected body part's textured base and outer geometry,
  still honoring the existing layer visibility toggles. A body part means both nested cuboids: its
  base skin and its outer clothing layer.
- Render faint, untextured edge outlines for each of the other five body parts so their spatial
  relationship remains visible without occluding or intercepting paint hits. Outline each part
  once using its base cuboid rather than duplicating edges for its outer layer, and use a dedicated
  line pipeline or equivalent renderer path with low alpha and no texture sampling or depth writes.
- Keep Shift-drag orbit fully available in solo mode. Ensure ordinary primary dragging paints newly
  exposed inner faces and Ctrl-click never begins a stroke.
- Reset solo mode when replacing the document, display the active solo part and Escape hint in the
  sidebar, and keep normal rendering and picking unchanged when no part is soloed.
- Extend picking and geometry helpers with an explicit body-part filter. Test entry and exit input
  behavior, selected-part picking, solid-geometry filtering, faint outline generation, layer
  toggles, and unchanged document state. Manually verify arm and leg interior faces can be painted,
  then run formatting, warnings-denied Clippy, and the full test suite.

## Add full-range tabbed color controls

Replaced the slider-only custom color section with HSV and RGBA/Hex tabs. The HSV tab provides a
full two-dimensional saturation/value field with hue and blend-alpha controls; the exact tab
provides byte-valued RGBA fields and validated `#RRGGBBAA` editing. Active color remains
unmultiplied RGBA8, transparent colors are shown over a checkerboard, and the tools pane now scrolls
at constrained heights. Added exact hex round-trip, invalid-input, and tab-state tests and visually
checked the full picker and minimum window layout.

### Original task

## Add full-range tabbed color controls

Replace the current coarse color-editing presentation with compact standard color-picker tabs while
keeping the active color as exact unmultiplied RGBA8.

- Add color-control tab state in `src/app.rs`: an HSV tab with a two-dimensional
  saturation/value field plus hue and alpha controls, and an RGBA tab with exact 0–255 channel
  entry and hexadecimal RGBA input.
- Use egui's existing color-picker facilities where practical, with `Alpha::OnlyBlend`; do not add
  a new GUI dependency merely to obtain a picker. Convert edits back to the app's `[u8; 4]`
  `active_color` without premultiplication or cumulative round-off.
- Keep transparent colors usable as erasers and show a checkerboard-backed active-color preview so
  alpha is legible. Preserve the existing quick swatches until the editable-palette task replaces
  their immutable storage.
- Adapt the computed tools-pane width and layout for every tab at the minimum supported window
  size.
- Test exact RGBA and hexadecimal round trips, alpha preservation, invalid hexadecimal handling,
  and tab changes that do not alter the active color. Manually verify the picker reaches arbitrary
  RGB and alpha values, then run formatting, warnings-denied Clippy, and the full test suite.

## Persist an editable quick palette

Converted the 16 quick swatches into app state with click-to-select and Shift-click-to-replace
behavior, checkerboard rendering for transparent slots, and concise interaction tooltips. Palette
preferences load from and save atomically to a versioned `~/.local/state/skindraw.json`, with safe
default fallback for absent, malformed, or unsupported state. Added state round-trip, atomic-file,
fallback, transparent assignment, and document-independence tests and visually verified the new
swatch rendering.

### Original task

## Persist an editable quick palette

Turn the 16 fixed quick swatches into user-editable application preferences.

- Move the current palette defaults from a compile-time constant into `SkinDrawApp` state. A normal
  primary click selects a swatch as the active color; Shift + primary click overwrites that slot
  with the current active RGBA color without changing the document.
- Store all 16 RGBA entries in `~/.local/state/skindraw.json`, creating the parent directory when
  needed. Load the state at startup and save palette changes atomically. Fall back safely to the
  built-in defaults when the file is absent or invalid, and preserve a versioned/extensible file
  shape for future application preferences.
- Keep palette preferences global across New and Open operations and out of the skin PNG, document
  dirty state, and undo/redo history.
- Retain clear selected-swatch styling, render transparent entries over a checkerboard, and add
  concise hover/help text explaining click and Shift-click behavior.
- Test default initialization, assignment including transparent colors, missing and malformed
  state-file fallback, save/restore round trips, and independence from document state. Manually
  verify persistence across an application restart, then run formatting, warnings-denied Clippy,
  and the full test suite.

## Flood-fill a contiguous face region

Added Brush and Fill tools with `B`/`F` shortcuts that defer to focused text editors and commit an
active stroke when switching. Fill performs one face-bounded four-connected exact-RGBA replacement
per primary click, records the operation through normal document history, supports transparent
outer faces, and uses the one-texel overlay as its seed preview. Added connectivity, diagonal,
boundary, alpha, transparent-region, no-op, history, tool-transition, and shortcut tests, and
visually verified face-only fill plus one-step undo in the native app.

### Original task

## Flood-fill a contiguous face region

Add a Fill tool that replaces one contiguous color region on the clicked model face with the active
color.

- Add a Brush/Fill tool selector in `src/app.rs`. Pressing `B` selects Brush and `F` selects Fill;
  do not consume these shortcuts while a text-entry control has keyboard focus. Changing tools
  finishes any active brush stroke.
- In Fill mode, one unmodified primary click on a valid hit performs exactly one fill; dragging
  over the view must not repeatedly start fills. Preserve Shift-drag orbit and Ctrl-click solo
  gestures.
- Implement a toolkit-independent four-connected flood fill in the editing core. Match the seed
  texel's complete RGBA value, constrain traversal to the selected part/layer/face `AtlasRect`, and
  never cross into an atlas-adjacent face or unused atlas pixels.
- Record every changed texel in one existing `Stroke` so a fill is one undo/redo entry, clears redo
  after a branched edit, updates dirty state normally, and becomes a no-op when the replacement
  color equals the seed color.
- Reuse `SkinDocument` mutation and history APIs rather than bypassing them. Ensure transparent
  regions on outer layers fill correctly and texture upload bounds include the complete changed
  region.
- Give Fill mode an appropriate cursor or seed preview and update sidebar instructions without
  restoring the removed whole-face tint.
- Add tests for bounded four-connectivity, diagonal separation, face-border containment, complete
  RGBA matching, transparent fills, no-op fills, keyboard tool switching, and undo/redo grouping.
  Manually verify fills on base, outer, soloed interior, and flipped atlas faces, then run
  formatting, warnings-denied Clippy, and the full test suite.

## Package and publish SkinDraw for Ubuntu desktops

Added `cargo-deb` metadata, a GNOME desktop entry, a matching Wayland application ID, an original
pixel-head SVG icon, and Ubuntu installation and release documentation. Added a tag-driven and
manually dispatchable GitHub Actions workflow with isolated publication permissions, version/tag
validation, current artifact actions, full Rust gates, Debian metadata and layout checks, and a
disposable Ubuntu 22.04 install/upgrade/removal test. Manually dispatched the non-publishing
workflow successfully and independently verified its `skindraw_0.1.0-1_amd64.deb` artifact and
SHA-256 report. Per the rollout decision, no tag or GitHub Release was created and `box` was not
modified.

### Original task

## Package and publish SkinDraw for Ubuntu desktops

Add a native amd64 Debian package and a tag-driven GitHub Actions release path so an Ubuntu user
can install SkinDraw system-wide with APT and launch it from GNOME without installing Rust or
checking out the repository. Build on GitHub's `ubuntu-22.04` runner so the glibc-linked binary runs
on Ubuntu 22.04 and newer, including the audited Ubuntu 24.04 GNOME/Wayland host `box`.

- Add the package metadata needed by `cargo-deb` to `Cargo.toml`: description, repository and
  maintainer information, Debian section and priority, automatic runtime dependencies, and explicit
  assets. Pin the CI-installed `cargo-deb` version rather than silently taking an arbitrary future
  release. Do not invent a project license; licensing remains a separate owner decision.
- Add `io.github.wbbradley.SkinDraw.desktop` with `Type=Application`, `Name=SkinDraw`,
  `Exec=skindraw`, `Icon=io.github.wbbradley.SkinDraw`, `Terminal=false`, and an appropriate
  graphics category. Do not advertise PNG MIME handling or use an `%f` argument unless startup is
  also taught to open a supplied file.
- Add a simple original SkinDraw application icon in a standard scalable or multi-resolution form.
  Package the binary at `/usr/bin/skindraw`, the desktop entry at
  `/usr/share/applications/io.github.wbbradley.SkinDraw.desktop`, and the icon under the matching
  hicolor icon-theme path in `/usr/share/icons/hicolor/`.
- Set the root eframe viewport's Wayland application ID in `src/main.rs` to exactly
  `io.github.wbbradley.SkinDraw`, matching the desktop filename without `.desktop`, so GNOME groups
  the running window with its launcher and icon. Preserve Wayland, X11 fallback, WGPU, and XDG
  portal behavior.
- Add `.github/workflows/release-linux.yml`. Run on version tags matching `v*` with
  `ubuntu-22.04`, use the locked Cargo dependency graph, install only the native build and packaging
  dependencies actually required, run formatting, tests, and warnings-denied Clippy, build the
  amd64 `.deb` with `cargo-deb`, and inspect its archive and control metadata before publication.
  Fail clearly when a `vX.Y.Z` tag does not match `package.version`.
- Give the release job `contents: write` only where publication requires it. Publish the generated
  `.deb` to the matching GitHub Release with generated notes. Also provide a safe, non-publishing
  `workflow_dispatch` path that uploads the `.deb` as a workflow artifact, allowing packaging to be
  tested without creating a release or tag.
- Document the maintainer flow for dispatching a package build and creating a matching version tag,
  plus the child-machine flow for downloading the release asset and running
  `sudo apt install ./...amd64.deb`. State the amd64 architecture and Ubuntu 22.04 baseline, explain
  that upgrades install a newer `.deb`, and leave Flatpak, Snap, a PPA, automatic client updates,
  arm64, and Rust installation on target machines out of scope.
- Validate desktop-entry syntax, icon lookup name, package contents and executable permissions, and
  Debian architecture, version, and dependency metadata. Test clean install and upgrade behavior in
  an appropriate Ubuntu environment. Record the exact build and runtime dependency lists learned
  from the package rather than relying on guessed libraries.
- After obtaining explicit approval for remote installation, smoke-test a CI-built package on
  `box` without adding a Rust toolchain: confirm it appears in GNOME application search with its
  icon, launches with correct dock/window grouping, renders through WGPU, and opens and saves through
  the portal. Similarly, do not push a public version tag or create a GitHub Release without explicit
  release authorization; the workflow and non-publishing artifact build can land independently.
- Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and the full
  Rust test suite. The implementation is ready when a manually dispatched workflow produces a
  correctly structured installable artifact; publication and target-machine installation remain
  approval-gated operations.

## Color rendering

Removed directional RGB multipliers from the model geometry so skin texels retain their exact sRGB
colors on every face instead of rendering darker than palette swatches. Kept the existing sRGB
texture pipeline, alpha behavior, preview, and solo guides unchanged. Added a regression test for
neutral face tinting, ran all 48 tests, and visually verified exact white rendering in the native
app.

### Original task

## Color rendering

Colors in the color palette are brighter than they render on the 3d skin in the drawing area. White from the underlying cuboids appears whiter than a pure white selected pain color, as a comparison.

## Right-click behavior

Added secondary-button color sampling to the 3D model view. Sampling uses the same visibility- and
solo-aware hit result as painting, copies the target texel's exact RGBA value into all active color
controls, and leaves the skin document and history unchanged. Added gesture and state regression
tests, passed all 50 tests, and verified transparent outer-layer and opaque base-layer sampling in
the native app.

### Original task

## Right-click behavior

Right-click should set the current color to the color of the pixel under the cursor, taking into
account solo mode, etc.

## Match 3D model colors to the color controls

Aligned the model renderer with egui's gamma-space framebuffer convention so opaque skin texels now
display identically to palette and active-color swatches. Added surface-format-aware compositing for
the less common sRGB target path, removed the obsolete shade vertex attribute, and added non-white
transfer regressions. All 51 tests and warnings-denied Clippy pass. Native screenshot measurements
matched exactly for gray and default red, and translucent outer-layer blending was verified over a
colored base.

### Original task

## Match 3D model colors to the color controls

Fix the remaining color-transfer mismatch that makes midtone skin colors render darker on the 3D
model than the same RGBA value in the palette and active-color swatch.

- Correct the GPU color path in `src/renderer.rs`. Egui normally renders into a non-sRGB
  `Rgba8Unorm` or `Bgra8Unorm` framebuffer using gamma-space UI colors, while the model currently
  samples an sRGB offscreen texture—decoding it to linear—and writes those linear values unchanged
  into that gamma-space framebuffer. Use a gamma-space `Rgba8Unorm` skin texture and offscreen
  target so opaque skin bytes and alpha blending follow the same convention as egui.
- Make final compositing respect `render_state.target_format`: pass gamma-space RGB through
  unchanged for non-sRGB surface formats, and convert gamma-space RGB to linear before writing to
  an sRGB surface so its hardware encoding produces the same displayed color. Select the
  appropriate WGSL fragment entry point when constructing the composite pipeline, mirroring
  egui-wgpu's handling of gamma and sRGB framebuffers.
- Preserve raw skin RGBA bytes, nearest-neighbor sampling, transparency discard behavior,
  outer-layer compositing, brush preview, solo guides, depth behavior, and texture subregion
  uploads. Do not add lighting or directional face tinting.
- Remove the now-redundant per-vertex `shade` attribute and associated shader plumbing so
  directional tint cannot accidentally return.
- Replace the previous format assertion with regression tests covering the actual transfer
  contract: gamma-space model/intermediate formats, pass-through compositing for `Rgba8Unorm` and
  `Bgra8Unorm`, linearized output for their sRGB variants, and neutral model vertices. Include
  representative dark, midtone, saturated, and white RGB values so a white-only check cannot mask
  the bug.
- Manually compare opaque palette colors and the active-color swatch against a painted face in one
  screenshot, away from the orange cursor preview. Representative midtones such as
  `[128, 128, 128, 255]` and the default red `[237, 28, 36, 255]` should visually match, allowing at
  most one 8-bit channel value of rounding. Separately verify translucent outer-layer colors
  composite correctly over the base layer; they are context-dependent and are not expected to
  match an isolated swatch.
- Run `cargo fmt --check`, warnings-denied Clippy, and the full test suite.
