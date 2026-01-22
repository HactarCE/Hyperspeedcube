FLAGS
- enabled right now?
- icon (alpha/numeric category of material design icons, card suites, gamepad, arrows, ghost, macOS modifier icon, some greek, square, rhombus, circle, polygons, dimensions, axis/rotations, axis locks, "set-")
  - must be invertible
- color
- name (force unique)
- comment
- computed from expression?
  - only sum-of-products?
  - e.g., SHIFT = LSHIFT or RSHIFT
  - e.g., ft_cube:n or ft_hypercube:3 (based on twist system ID or puzzle ID)
  - e.g., "no grip" or grip="R"
  - regex support? `twists="ft_cubic:.*"`
  - hyperpuzzlescript???
- show in statusbar?



gather keybinds first, THEN execute (don't change flags in middle of evaluating)
- when gathering, resolve "toggle" to "enable vs. disable"
- at the end of the day, ordering is top-to-bottom
- execute all keybinds that match (until a "stop processing keybinds") but ignore twists after the first


display active flags in statusbar (or similar)


- Modifiers
  - LSHIFT sets "SHIFT" flag while held
  - RSHIFT sets "SHIFT" flag while held
  - LSHIFT sets "LSHIFT" flag while held
  - RSHIFT sets "RSHIFT" flag while held




A, B, C, D, E


BC,ACD




KEYBIND FOLDER
- name
- optional color (shows in keybind ref)
- optional comment
- list of keybinds
- list of subfolders
- boolean expression based on modes (+ requires parents to be active)
  - sum of products
  - option for "always"
  - probably UI for "any of these modes" (simple OR)
    - icons, hover for name (letters, digits, macOS modifier icons, others?)
  - maybe UI for sum-of-products?
  - maybe UI for typing in an arbitrary boolean expression? (like filters DSL)
  - can depend on twist system ID/generator
- can be disabled with checkbox

KEYBIND
- can be disabled with checkbox (empty or X; grey out keybind when disabled)
- optional color?
- maybe also boolean expression?
- maybe also comment
- key repeat?



"while held" keybind that sets, resets, or toggles a mode flag
- decide what to do when pressed, even if modes change



- twists can have multipliers

"multiply while held"
"repeat last action N times" (negative N to do inverse)
"do inverse while held"
"undo + do inverse" (go back and forth)


DEBUG VIEW
- press a key, see the resolution logic (which keybinds match vs. are disabled, and priority)
  - show folder structure
- click on keybinds ref to toggle that key
- checkbox to disable keyboard input
- customize keyboard layout?


whole window should be searchable?


add a "slash" icon that can be displayed over other icons. (yes, it needs opaque background color. this is surely doable somehow.)




B set flag F when pressed


A down, B down, B up, A up



MODE (global boolean flag)
- Default
- RKT
- 2-gen




AXIOMS
- modifier keys aren't special


define global modifier for global keybinds















GLOBAL KEYBINDS
- Z = undo
- shift+Z = redo
- Y = redo
- F = full scramble
- R = reset
- B = blindfold
- Q = exit
- O = open
- S = save
- shift + S = save as

- shift + other keys = toggle panes?
  - C = colors
  - P = command palette
  - L = catalog ("listing")
  - F = filters
  - V = view
  - A = animation
  - I = interaction
  - T = reopen? timer?
  - K = puzzle keybinds
- ctrl + N = new puzzle pane
