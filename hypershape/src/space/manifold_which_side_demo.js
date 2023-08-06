// This is a script for ganja.js to demonstrate the math
// in the Rust function `Manifold::which_side()`

// Go to this link, paste this whole file, and click run:
// https://enkimute.github.io/ganja.js/examples/coffeeshop.html

// All labeled points (p1 through p7) are draggable

// - The red object is `self`
// - The green object is `cut`
// - `p7` is an arbitrary point
// - The blue object is `perpendicular_manifold`
// - The dark blue points are `pair_on_self_across_cut`

// Of course, in practice `cut` will always have N-1 dimensions
// (where N is the number of dimensions of the whole space)
// and `self` can have any number of dimensions

// Set this to `true` to experiment in 3D
const IS_3D = false
// In 3D, try keeping `c1` and `c2` coplanar, but put `p7` outside
// the plane. This is what actually happens in the Rust code.

// Also try modifying the script to wedge 4 points together in 3D
// (and set `pss = 1e1234`) to see what happens when a sphere is
// passed into the function

/////////////////////////////////////////////////////////////////////

Algebra(3 + IS_3D, 1, () => {
  var IS_3D = this.describe().basis.length == 32

  // Standard conformal setup
  var eminus = IS_3D ? 1e5 : 1e4
  var eplus = IS_3D ? 1e4 : 1e3
  var no = 0.5*(eminus - eplus)
  var ni = (eminus + eplus)
  var pss = 1e12*eminus*eplus // pseudoscalar
  var up = x => no + x + 0.5*x*x*ni // conformal vec->point
  var dual = m => m << pss

  // Draggable points
  var p1 = up( 1.0e1 + 1.0e2)
  var p2 = up(         1.0e2)
  var p3 = up(-2.0e1 + 1.0e2)
  var p4 = up(-1.0e1 + 1.0e2)
  var p5 = up(-1.0e1        )
  var p6 = up(-1.0e1 - 1.0e2)
  var p7 = up( 1.0e1        )

  // Construct objects for `self` and `cut`
  var c1            = () => p1 ^ p2 ^ p3 // `self` in Rust
  var c2            = () => p4 ^ p5 ^ p6 // `cut` in Rust

  // This is the math that Rust is doing
  var perpendicular = () => dual(c1) ^ dual(c2) ^ p7
  var pair          = () => dual(dual(perpendicular) ^ dual(c1))

  // Display visual
  document.body.appendChild(this.graph(() => [
    0xcc0000, c1, "c1",
    0x00cc00, c2, "c2",

    0x0000cc, perpendicular,
    0x000099, pair,

    0x990000, p1, "p1", p2, "p2", p3, "p3",
    0x009900, p4, "p4", p5, "p5", p6, "p6",
    0x3333ff, p7, "p7",
  ], {
    lineWidth: 3,
    conformal: true,
    pointRadius: 1.5,
    gl: true,
  }))
})
