Each piece tracks its attitude as a tuple `(doctrinaire_grip_group_element, jumble_offset_motor)`. (This is already being implemented.)

New design:
- `jumble_offset_motor` must track the transform history that got it there.
- 3D gizmo geometry is recalculated whenever a move is done to the puzzle.
  1. Gather all unique `jumble_offset_motor`s.
  2. Transform all axes by each `jumble_offset_motor`.
  3. Filter for unblocked axes. (could be optional! try showing all gizmos and see how it feels)
  4. Generate gizmo polyhedron. Each gizmo face is carved by axes within its own system (whether or not they are blocked) and carved by unblocked axes outside its own system.
- Maybe only refresh gizmos on mouse move, so that double-click is ok.
- Improve backface gizmos
  - Dashed/dotted line if backface
  - Reverse click direction if backface


open question: how to prevent infinite gizmo faces?




@_Uj = move the whole axis system by `Uj` transform

These moves are implicitly added when necessary for jumbling.
