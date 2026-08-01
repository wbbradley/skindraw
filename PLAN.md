# Next Up

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
