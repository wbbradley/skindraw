# Next Up

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
