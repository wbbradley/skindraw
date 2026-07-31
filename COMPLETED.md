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
