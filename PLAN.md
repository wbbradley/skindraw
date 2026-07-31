# Next Up

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
