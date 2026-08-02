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


## Orbit by dragging empty model-view space

Implemented drag-origin-aware viewport gestures: a primary drag that begins off the character now orbits, while drags that begin on the character retain paint behavior. Shift-drag orbit and Ctrl-click solo behavior remain intact, with focused gesture regressions.

### Original task

## Rotations when clicking in the dead space

Let's make it so that clicking and dragging off the character, causes orbit rotations, instead of
just doing nothing.


## Prevent macOS file-dialog drag/drop reentrancy crashes

Replaced blocking Open and Save As dialogs with asynchronous native sheets so file-panel drag/drop no longer nests a macOS event loop inside winit. Preserved dirty-document confirmations and deferred actions across saves, including cancellation and error paths, and added regressions for open and save-dialog completion.

### Original task

## Fix file open dragging bug

When dragging .png from another folder into the file open dialog I saw a crash. Standard macOS behavior is to navigate the open dialog to that location. This crash occurred when I dropped a file on the dialog, I think.


~/src/skindraw$ cargo run
   Compiling skindraw v0.1.0 (/Users/wbbradley/src/skindraw)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.67s
     Running `target/debug/skindraw`

thread 'main' (10369477) panicked at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/event_handler.rs:135:17:
tried to handle event while another event is currently being handled
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'main' (10369477) panicked at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/panicking.rs:225:5:
panic in a function that cannot unwind
stack backtrace:
   0:        0x103728d7c - std[7d7552da0923e8b2]::backtrace_rs::backtrace::libunwind::trace
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/../../backtrace/src/backtrace/libunwind.rs:117:9
   1:        0x103728d7c - std[7d7552da0923e8b2]::backtrace_rs::backtrace::trace_unsynchronized::<std[7d7552da0923e8b2]::sys::backtrace::_print_fmt::{closure#1}>
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/../../backtrace/src/backtrace/mod.rs:66:14
   2:        0x103728d7c - std[7d7552da0923e8b2]::sys::backtrace::_print_fmt
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/sys/backtrace.rs:74:9
   3:        0x103728d7c - <<std[7d7552da0923e8b2]::sys::backtrace::BacktraceLock>::print::DisplayBacktrace as core[4b39a0a778b8475a]::fmt::Display>::fmt
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/sys/backtrace.rs:44:26
   4:        0x10373c080 - <core[4b39a0a778b8475a]::fmt::rt::Argument>::fmt
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/fmt/rt.rs:152:76
   5:        0x10373c080 - core[4b39a0a778b8475a]::fmt::write
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/fmt/mod.rs:1687:22
   6:        0x10372cc80 - std[7d7552da0923e8b2]::io::default_write_fmt::<std[7d7552da0923e8b2]::sys::stdio::unix::Stderr>
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/io/mod.rs:621:11
   7:        0x10372cc80 - <std[7d7552da0923e8b2]::sys::stdio::unix::Stderr as std[7d7552da0923e8b2]::io::Write>::write_fmt
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/io/mod.rs:1976:13
   8:        0x1037149b4 - <std[7d7552da0923e8b2]::sys::backtrace::BacktraceLock>::print
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/sys/backtrace.rs:47:25
   9:        0x1037149b4 - std[7d7552da0923e8b2]::panicking::default_hook::{closure#0}
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:292:27
  10:        0x10372309c - std[7d7552da0923e8b2]::panicking::default_hook
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:319:9
  11:        0x1037233c4 - std[7d7552da0923e8b2]::panicking::panic_with_hook
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:825:13
  12:        0x103714a8c - std[7d7552da0923e8b2]::panicking::panic_handler::{closure#0}
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:691:13
  13:        0x103709d64 - std[7d7552da0923e8b2]::sys::backtrace::__rust_end_short_backtrace::<std[7d7552da0923e8b2]::panicking::panic_handler::{closure#0}, !>
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/sys/backtrace.rs:182:18
  14:        0x103715294 - __rustc[8068f81614cfe5c]::rust_begin_unwind
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:689:5
  15:        0x1037b2f68 - core[4b39a0a778b8475a]::panicking::panic_nounwind_fmt::runtime
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/panicking.rs:122:22
  16:        0x1037b2f68 - core[4b39a0a778b8475a]::panicking::panic_nounwind_fmt
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/intrinsics/mod.rs:2448:9
  17:        0x1037b2ef0 - core[4b39a0a778b8475a]::panicking::panic_nounwind
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/panicking.rs:225:5
  18:        0x1037b3044 - core[4b39a0a778b8475a]::panicking::panic_cannot_unwind
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/panicking.rs:337:5
  19:        0x10286929c - <Closure as block2::traits::IntoBlock<(),R>>::__get_invoke_stack_block::invoke::h57a5880509d47996
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/block2-0.5.1/src/traits.rs:103:17
  20:        0x18105ead0 - <unknown>
  21:        0x18105ea10 - <unknown>
  22:        0x18105d844 - <unknown>
  23:        0x1811301c4 - <unknown>
  24:        0x1828a7b44 - <unknown>
  25:        0x18291bbec - <unknown>
  26:        0x18570b188 - <unknown>
  27:        0x188649ed0 - <unknown>
  28:        0x1886502ac - <unknown>
  29:        0x1810f38b0 - <unknown>
  30:        0x18105f4a8 - <unknown>
  31:        0x18105f3d0 - <unknown>
  32:        0x18105dd98 - <unknown>
  33:        0x1811301c4 - <unknown>
  34:        0x18de43560 - <unknown>
  35:        0x18de468bc - <unknown>
  36:        0x18dfd014c - <unknown>
  37:        0x185b3835c - <unknown>
  38:        0x18548c084 - <unknown>
  39:        0x1860218b0 - <unknown>
  40:        0x1860215bc - <unknown>
  41:        0x1856b4324 - <unknown>
  42:        0x1856b2f4c - <unknown>
  43:        0x1856b2ef8 - <unknown>
  44:        0x1856b27c4 - <unknown>
  45:        0x1856b267c - <unknown>
  46:        0x186389b14 - <unknown>
  47:        0x102f17164 - <() as objc2::encode::EncodeArguments>::__invoke::hc956a246ee58a42f
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.6.4/src/encode.rs:433:26
  48:        0x102f18e10 - objc2::runtime::message_receiver::msg_send_primitive::send::hf27c0d25d4e7f7b0
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.6.4/src/runtime/message_receiver.rs:172:18
  49:        0x102f13c20 - objc2::runtime::message_receiver::MessageReceiver::send_message::h29c43a7ab2f48cf3
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.6.4/src/runtime/message_receiver.rs:432:38
  50:        0x102810c1c - <MethodFamily as objc2::__macro_helpers::msg_send_retained::MsgSend<Receiver,Return>>::send_message::h1b25bd3eef418fd0
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.6.4/src/__macro_helpers/msg_send_retained.rs:35:28
  51:        0x10281977c - objc2_app_kit::generated::__NSSavePanel::NSSavePanel::runModal::h43dd408ae09d38d5
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.6.4/src/macros/extern_methods.rs:266:14
  52:        0x102586a44 - rfd::backend::macos::file_dialog::panel_ffi::Panel::run_modal::h3e20e581d257df6a
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rfd-0.17.2/src/backend/macos/file_dialog/panel_ffi.rs:76:29
  53:        0x102588c4c - rfd::backend::macos::file_dialog::<impl rfd::backend::FilePickerDialogImpl for rfd::file_dialog::FileDialog>::pick_file::{{closure}}::{{closure}}::hd737df70bd0bbb09
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rfd-0.17.2/src/backend/macos/file_dialog.rs:24:26
  54:        0x102588ef8 - rfd::backend::macos::utils::run_on_main::h476216c1773d6138
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rfd-0.17.2/src/backend/macos/utils.rs:22:9
  55:        0x102588bfc - rfd::backend::macos::file_dialog::<impl rfd::backend::FilePickerDialogImpl for rfd::file_dialog::FileDialog>::pick_file::{{closure}}::hc32aa9fe685a59a1
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rfd-0.17.2/src/backend/macos/file_dialog.rs:21:13
  56:        0x10258acc8 - objc2::rc::autorelease::autoreleasepool::hfd6198412ba95279
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.6.4/src/rc/autorelease.rs:453:15
  57:        0x10258616c - rfd::backend::macos::file_dialog::<impl rfd::backend::FilePickerDialogImpl for rfd::file_dialog::FileDialog>::pick_file::hcf430774510a2709
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rfd-0.17.2/src/backend/macos/file_dialog.rs:20:9
  58:        0x1025860ac - rfd::file_dialog::FileDialog::pick_file::h5ab5d407ce837689
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rfd-0.17.2/src/file_dialog.rs:123:9
  59:        0x10257968c - skindraw::app::SkinDrawApp::open::h528beee5005e69eb
                               at /Users/wbbradley/src/skindraw/src/app.rs:676:79
  60:        0x102578ae8 - skindraw::app::SkinDrawApp::execute_action::h2a9218e5144dd4c6
                               at /Users/wbbradley/src/skindraw/src/app.rs:658:42
  61:        0x102578bbc - skindraw::app::SkinDrawApp::request_action::h75e8e88a7382f4d1
                               at /Users/wbbradley/src/skindraw/src/app.rs:648:18
  62:        0x10257a518 - skindraw::app::SkinDrawApp::toolbar::h534118dad9b4588a
                               at /Users/wbbradley/src/skindraw/src/app.rs:206:18
  63:        0x102577a24 - <skindraw::app::SkinDrawApp as eframe::epi::App>::ui::h86a996264a125e30
                               at /Users/wbbradley/src/skindraw/src/app.rs:815:14
  64:        0x1025e3268 - eframe::native::epi_integration::EpiIntegration::update::{{closure}}::he0da23d6958dd740
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/epi_integration.rs:295:29
  65:        0x102f9592c - egui::context::Context::run_ui_dyn::{{closure}}::h7f902f2698bf3075
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.35.0/src/context.rs:798:17
  66:        0x102fba950 - egui::context::Context::run_dyn::he94a6ebb81498f40
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.35.0/src/context.rs:842:13
  67:        0x102f95718 - egui::context::Context::run_ui_dyn::ha215e55bb9e43027
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.35.0/src/context.rs:787:14
  68:        0x1025bfb58 - egui::context::Context::run_ui::hab16e4f61524513d
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.35.0/src/context.rs:781:14
  69:        0x1025e2db4 - eframe::native::epi_integration::EpiIntegration::update::h571ff981371caf6f
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/epi_integration.rs:279:41
  70:        0x1025cca30 - eframe::native::wgpu_integration::WgpuWinitRunning::run_ui_and_paint::h27b0c04a1d9c1259
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/wgpu_integration.rs:680:25
  71:        0x1025c8018 - <eframe::native::wgpu_integration::WgpuWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint::h713513d40491c6ad
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/wgpu_integration.rs:416:21
  72:        0x1025df810 - <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event::{{closure}}::hd707abff20f3e9ca
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/run.rs:363:36
  73:        0x1025de4f8 - eframe::native::event_loop_context::with_event_loop_context::ha42615f1891181d2
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/event_loop_context.rs:53:5
  74:        0x1025df7a0 - <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event::h965d42528f467966
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/run.rs:360:9
  75:        0x1025e1be0 - winit::event_loop::dispatch_event_for_app::h316e694af159bda2
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/event_loop.rs:642:56
  76:        0x1025e1be0 - winit::platform::run_on_demand::EventLoopExtRunOnDemand::run_app_on_demand::{{closure}}::hf4ab17cc2bbdb862
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform/run_on_demand.rs:76:13
  77:        0x1025c4f98 - winit::platform_impl::macos::event_loop::map_user_event::{{closure}}::h74c63353bfa7c978
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/event_loop.rs:174:22
  78:        0x102870c44 - <alloc::boxed::Box<F,A> as core::ops::function::FnMut<Args>>::call_mut::hbce68319d5eef658
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/alloc/src/boxed.rs:2256:9
  79:        0x10287581c - winit::platform_impl::macos::event_handler::EventHandler::handle_event::h82b070d885832aeb
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/event_handler.rs:125:17
  80:        0x102898b20 - winit::platform_impl::macos::app_state::ApplicationDelegate::handle_event::h3f73bb0d3d195347
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/app_state.rs:324:36
  81:        0x10289a930 - winit::platform_impl::macos::app_state::ApplicationDelegate::cleared::h2f2e772a9d15588b
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/app_state.rs:386:18
  82:        0x1028884d0 - winit::platform_impl::macos::observer::control_flow_end_handler::{{closure}}::h959e20e686e2047b
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/observer.rs:84:80
  83:        0x102888330 - winit::platform_impl::macos::observer::control_flow_handler::{{closure}}::h80d7e149cc8aae79
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/observer.rs:46:9
  84:        0x102857c24 - std::panicking::catch_unwind::do_call::hdc9824567d55b879
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/panicking.rs:581:40
  85:        0x102878f68 - ___rust_try
  86:        0x1028740e0 - std::panicking::catch_unwind::h01a200bd1ec0ed66
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/panicking.rs:544:19
  87:        0x1028740e0 - std::panic::catch_unwind::h844f05abdef8235a
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/panic.rs:359:14
  88:        0x102895b70 - winit::platform_impl::macos::event_loop::stop_app_on_panic::hf25ad34c60d4a6a2
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/event_loop.rs:444:11
  89:        0x102888144 - winit::platform_impl::macos::observer::control_flow_handler::h7e96b6cb799b5622
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/observer.rs:44:5
  90:        0x1028883a4 - winit::platform_impl::macos::observer::control_flow_end_handler::he3531071011d25cd
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/observer.rs:79:9
  91:        0x18105e314 - <unknown>
  92:        0x18105e210 - <unknown>
  93:        0x18105d8bc - <unknown>
  94:        0x1811301c4 - <unknown>
  95:        0x18de43560 - <unknown>
  96:        0x18de468bc - <unknown>
  97:        0x18dfd014c - <unknown>
  98:        0x185b3835c - <unknown>
  99:        0x18548c084 - <unknown>
 100:        0x1860218b0 - <unknown>
 101:        0x1860215bc - <unknown>
 102:        0x18547f13c - <unknown>
 103:        0x1028d7098 - <() as objc2::encode::EncodeArguments>::__invoke::hbf1ced3cd8c99e5f
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.5.2/src/encode.rs:437:26
 104:        0x1028d8284 - objc2::runtime::message_receiver::msg_send_primitive::send::hd18476ad928983db
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.5.2/src/runtime/message_receiver.rs:173:18
 105:        0x1028ce5f4 - objc2::runtime::message_receiver::MessageReceiver::send_message::h08f6d660dbb42ed5
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.5.2/src/runtime/message_receiver.rs:433:38
 106:        0x1028c3d00 - objc2::__macro_helpers::msg_send::MsgSend::send_message::h55f9d9625e3868c2
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.5.2/src/__macro_helpers/msg_send.rs:27:31
 107:        0x1028c3324 - objc2_app_kit::generated::__NSApplication::NSApplication::run::h066a24ac0b9ec049
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.5.2/src/macros/extern_methods.rs:247:14
 108:        0x1025c580c - winit::platform_impl::macos::event_loop::EventLoop<T>::run_on_demand::{{closure}}::{{closure}}::h779e259732b63572
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/event_loop.rs:299:35
 109:        0x1025bfd20 - objc2::rc::autorelease::autoreleasepool::h078e0ee02844783c
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.5.2/src/rc/autorelease.rs:438:15
 110:        0x1025c55c0 - winit::platform_impl::macos::event_loop::EventLoop<T>::run_on_demand::{{closure}}::h73550740fe8abaa5
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/event_loop.rs:285:13
 111:        0x1025c61d8 - winit::platform_impl::macos::event_handler::EventHandler::set::h2c57085a16fbfe32
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/event_handler.rs:98:9
 112:        0x1025c1fbc - winit::platform_impl::macos::app_state::ApplicationDelegate::set_event_handler::h4e0794702d866066
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/app_state.rs:193:36
 113:        0x1025c541c - winit::platform_impl::macos::event_loop::EventLoop<T>::run_on_demand::h0ecdd39268a75e60
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/macos/event_loop.rs:284:23
 114:        0x1025def6c - <winit::event_loop::EventLoop<T> as winit::platform::run_on_demand::EventLoopExtRunOnDemand>::run_on_demand::h806ff24e14541d12
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform/run_on_demand.rs:89:25
 115:        0x1025e1a58 - winit::platform::run_on_demand::EventLoopExtRunOnDemand::run_app_on_demand::h0388a14c1f0e8e38
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform/run_on_demand.rs:75:14
 116:        0x1025e4254 - eframe::native::run::run_and_return::ha3751d72b8a09f75
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/run.rs:380:16
 117:        0x1025e6578 - eframe::native::run::run_wgpu::{{closure}}::h4d055ce7b7535ef4
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/run.rs:451:13
 118:        0x1025e46b8 - eframe::native::run::with_event_loop::{{closure}}::h59abf388680cb9c0
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/run.rs:73:12
 119:        0x1025b866c - std::thread::local::LocalKey<T>::try_with::h38ef429e1de215c4
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/thread/local.rs:462:12
 120:        0x1025b8170 - std::thread::local::LocalKey<T>::with::h7f74d2eb904f174f
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/thread/local.rs:426:20
 121:        0x1025e4540 - eframe::native::run::with_event_loop::h34d91dcc36eabdfb
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/run.rs:63:16
 122:        0x1025e61d0 - eframe::native::run::run_wgpu::hf7f62a98e6ced7b7
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/native/run.rs:448:16
 123:        0x1025e20e0 - eframe::run_native_ext::hd7ab2f67a35468c2
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/lib.rs:324:13
 124:        0x1025e1e20 - eframe::run_native::h9e9f5d02caf334ee
                               at /Users/wbbradley/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.35.0/src/lib.rs:293:5
 125:        0x102540fc4 - skindraw::main::hc4b85ce8c9cd4e31
                               at /Users/wbbradley/src/skindraw/src/main.rs:6:5
 126:        0x102541098 - core::ops::function::FnOnce::call_once::h4e129bc1ce279a69
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/ops/function.rs:250:5
 127:        0x102540d94 - std::sys::backtrace::__rust_begin_short_backtrace::h4d2203f691605bcf
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/sys/backtrace.rs:166:18
 128:        0x10254193c - std::rt::lang_start::{{closure}}::h56c39411ff7a8310
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/rt.rs:206:18
 129:        0x103722644 - <&dyn core[4b39a0a778b8475a]::ops::function::Fn<(), Output = i32> + core[4b39a0a778b8475a]::marker::Sync + core[4b39a0a778b8475a]::panic::unwind_safe::RefUnwindSafe as core[4b39a0a778b8475a]::ops::function::FnOnce<()>>::call_once
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/ops/function.rs:287:21
 130:        0x103722644 - std[7d7552da0923e8b2]::panicking::catch_unwind::do_call::<&dyn core[4b39a0a778b8475a]::ops::function::Fn<(), Output = i32> + core[4b39a0a778b8475a]::marker::Sync + core[4b39a0a778b8475a]::panic::unwind_safe::RefUnwindSafe, i32>
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:581:40
 131:        0x103722644 - std[7d7552da0923e8b2]::panicking::catch_unwind::<i32, &dyn core[4b39a0a778b8475a]::ops::function::Fn<(), Output = i32> + core[4b39a0a778b8475a]::marker::Sync + core[4b39a0a778b8475a]::panic::unwind_safe::RefUnwindSafe>
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:544:19
 132:        0x103722644 - std[7d7552da0923e8b2]::panic::catch_unwind::<&dyn core[4b39a0a778b8475a]::ops::function::Fn<(), Output = i32> + core[4b39a0a778b8475a]::marker::Sync + core[4b39a0a778b8475a]::panic::unwind_safe::RefUnwindSafe, i32>
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panic.rs:359:14
 133:        0x103722644 - std[7d7552da0923e8b2]::rt::lang_start_internal::{closure#0}
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/rt.rs:175:24
 134:        0x103722644 - std[7d7552da0923e8b2]::panicking::catch_unwind::do_call::<std[7d7552da0923e8b2]::rt::lang_start_internal::{closure#0}, isize>
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:581:40
 135:        0x103722644 - std[7d7552da0923e8b2]::panicking::catch_unwind::<isize, std[7d7552da0923e8b2]::rt::lang_start_internal::{closure#0}>
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:544:19
 136:        0x103722644 - std[7d7552da0923e8b2]::panic::catch_unwind::<std[7d7552da0923e8b2]::rt::lang_start_internal::{closure#0}, isize>
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panic.rs:359:14
 137:        0x103722644 - std[7d7552da0923e8b2]::rt::lang_start_internal
                               at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/rt.rs:171:5
 138:        0x10254190c - std::rt::lang_start::hc98ec76e292e80c8
                               at /Users/wbbradley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/rt.rs:205:5
 139:        0x10254102c - _main
thread caused non-unwinding panic. aborting.
Abort trap: 6              cargo run

## Always-visible viewing orientation

Added a persistent viewport badge derived from camera yaw that reports the nearest Front, Back,
Left, or Right side while orbiting.

### Original task

* Let's have some always visible text that tells you whether you are looking at the front, back,
  left, or right of the player.

## Exploded body-part layout

Added a paintable Exploded layout that substantially separates every cuboid while keeping all
layers visible and interactive. Rendering, ray picking, and brush previews share the same translated
geometry; the camera widens to frame it, Solo and Exploded remain mutually exclusive, and Escape
returns to Joined layout.

### Original task

* Let's have an alternative to solo mode that still shows all the cuboids, and allows you to paint
  on them, however, instead of being flush together, they are all split apart by a considerable
  distance so that the tools can reach into the interior faces more easily.

## Per-texel Gaussian HSV jitter

Added persisted hue, saturation, and value standard-deviation controls. Brush interpolation and
flood fill independently sample additive normal offsets for every affected texel, wrapping hue,
clamping saturation/value, converting back to RGB, and preserving alpha exactly.

### Original task

* Let's change the way the color selection interacts with the brush. Let's have a "random jitter"
  factor that is tweakable in the side pane. This jitter will have three dimensions, one each for
  hue, saturation, and value. They represent stddev against those parameters in an additive fashion.
  So, for each pixel being set (including in fill operations), we will take the current color, find
  random (normal dist) offsets for that color's HSV apply those offsets, then convert that back to
  RGB for the final pixel setting operation.

## Release version 0.1.3

Updated the README and changelog, verified the optimized release build, bumped Cargo to 0.1.3,
created the annotated `v0.1.3` tag, and pushed both `main` and the tag to GitHub.

### Original task

* Finally, let's bump the version, and push to github with a new tag, etc..
