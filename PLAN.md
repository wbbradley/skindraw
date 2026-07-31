# Next Up

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
