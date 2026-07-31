# Next Up

## Build the 3D Minecraft skin editor

Replace the Rust binary stub with a native macOS and Linux app for making and editing Minecraft
Java Edition skins. Use `eframe`/`egui` for the app shell and controls, and use the `wgpu` renderer
exposed by `eframe` plus an `egui_wgpu` paint callback for the 3D view. This keeps the app small and
UI-led while giving the model view direct access to a depth buffer, skin texture, and render
pipeline. Do not use Bevy for the first build: Bevy supplies mesh picking, but its game loop and ECS
add costs that the six-part model and direct CPU ray tests do not need.

Research behind the choice:

- `eframe` runs egui apps as native desktop apps and enables its `wgpu`, Wayland, and X11 paths by
  default: <https://docs.rs/eframe/latest/eframe/>
- `egui_wgpu::Callback` supports custom `wgpu` drawing inside an egui region:
  <https://docs.rs/egui-wgpu/latest/egui_wgpu/struct.Callback.html>
- `wgpu::Queue::write_texture` can upload changed pixel regions without rebuilding the model:
  <https://docs.rs/wgpu/latest/wgpu/struct.Queue.html#method.write_texture>
- Bevy's mesh picking can cast scene rays, but the docs also point complex tools to direct ray casts:
  <https://bevy.org/examples-webgpu/picking/mesh-picking/>
- Minecraft Java skins use either the Classic/Standard or Slim character model:
  <https://help.minecraft.net/hc/en-us/articles/4408894664461-Make-a-Custom-Skin-in-Minecraft-Java-Edition>

### Product scope

- Start with a valid blank or bundled 64×64 RGBA skin and let the user create a new Classic or Slim
  skin. Treat 64×64 Java Edition PNG files as the supported file format. Reject other sizes with a
  clear error; leave 64×32 legacy conversion and Bedrock skin packs out of scope.
- Open a 64×64 PNG from disk, keep the CPU pixel buffer as the source of truth, and save or save as a
  lossless 64×64 RGBA PNG. Track unsaved edits and ask before New, Open, or Quit discards them.
- Render the current skin on the head, torso, two arms, and two legs with nearest-neighbor sampling.
  Implement the official 64×64 atlas layout for each face, including separate left and right limbs.
  Let the user switch between Classic four-pixel arms and Slim three-pixel arms without changing the
  stored pixels.
- Render both the base skin and the slightly larger hat, jacket, sleeve, and trouser layers. Add
  simple toggles for the base and outer layers so the user can reach pixels hidden by an outer
  layer. Preserve alpha in the outer layer.
- Keep an orthographic camera aimed at the model. Use primary-button drag to paint. While Space is
  held, use primary-button drag to change yaw and pitch instead; clamp pitch before the view flips,
  allow full yaw, and never add roll. Keep all input inside the 3D view so sidebar use does not move
  or paint the model.
- Cast a CPU ray from the pointer through the camera and test the visible model boxes from nearest to
  farthest. Convert the winning face hit to the exact atlas texel with explicit per-face UV maps.
  Account for model pose, Classic/Slim arm width, base versus outer layer, face direction, and
  mirrored atlas faces.
- Paint on press and drag. Interpolate between sampled texels so a quick drag leaves no gaps. Clip a
  brush stamp to the hit face's atlas rectangle so larger brushes cannot alter a nearby, unrelated
  atlas island. Refresh the GPU texture after each stroke or changed region so the model gives
  immediate feedback.
- Put a fixed color palette in a right sidebar. Show the active color, allow a swatch click to select
  it, and include an RGBA color editor for custom colors. Add clear brush-size choices measured in
  skin pixels, with sizes 1, 2, and 4. Show a small cursor or face highlight on a valid hit.
- Add undo and redo at stroke granularity, including shortcuts, so one drag is one history entry.
  Include common desktop shortcuts for New, Open, Save, Save As, Undo, and Redo, and show file or
  validation failures in the app rather than only on stderr.

### Code structure

- Keep `src/main.rs` limited to native startup and split app state, skin PNG I/O, atlas maps and model
  data, camera and pointer interaction, brush/history logic, and `wgpu` rendering into focused
  modules.
- Keep ray casting, face-to-atlas mapping, and brush edits independent from egui and GPU types where
  practical. Use small data types for model part, layer, face, texel, and Classic/Slim model kind so
  code cannot mix those values by accident.
- Generate the block model from code. Store one shared CPU atlas and one GPU texture; do not add a
  general scene format, physics engine, or model asset pipeline.
- Use `rfd` or another small native dialog crate for Open and Save As. Add only the math, PNG, buffer
  cast, and error crates needed by the app.

### Tests and completion checks

- Add table-driven tests for every face of every body part in both Classic and Slim modes. Test known
  ray hits from front, back, left, right, above, and below, and assert the expected atlas texel and
  layer.
- Test ray miss and nearest-layer behavior, pitch bounds, brush clipping, stroke interpolation,
  transparent outer-layer edits, undo/redo grouping, dirty state, invalid PNG rejection, and PNG
  load/save round trips.
- Run the standard Rust formatter, unit tests, and Clippy with warnings denied.
- Manually check a debug build on Linux and macOS: create and open a skin, paint all visible model
  faces at each brush size, rotate with Space-drag, edit base and outer layers, undo and redo, switch
  arm type, save, reopen the PNG, and confirm the pixels and alpha remain unchanged.
- The task is complete when the native app starts on both targets, all controls work with no panic,
  direct painting selects the texel under the pointer from several camera angles, and a saved PNG can
  be selected as a Java Edition skin.
