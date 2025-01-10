local gizmo_size = 1

-- TODO: add face turns

function def_simplex(letter, depths)
  depths = lib.utils.concatseq({INF}, depths)

  puzzles:add{
    id = '4_simplex_' .. string.lower(letter),
    version = '0.1.0',
    name = "4-Simplex " .. letter,
    ndim = 4,
    colors = '4_simplex',
    build = function(self)
      local sym = cd'a4'
      local shape = lib.symmetries.a4.simplex_4d()
      self:carve(shape:iter_poles())

      local ooox = sym.ooox.unit

      -- Define twists
      local axes = self.axes:add(shape:iter_vertices(), depths)
      local a1 = axes[5]
      local a2 = sym:thru(4):transform(a1)
      local a3 = sym:thru(3):transform(a2)
      local a4 = sym:thru(2):transform(a3)
      local t = sym:thru(2, 1)
      for _, axis1, axis2, twist_transform in sym.chiral:orbit(a1, a2, t) do
        self.twists:add(axis1, twist_transform, {
          name = axis2,
          gizmo_pole_distance = 2/sqrt(3) * gizmo_size,
        })
      end

      local ridge = a2.vector + a3.vector -- ridge orthogonal to `a1`
      local t = sym:thru(3, 1) -- rot{fix = a1.vector ^ ridge, angle = PI}
      for t, axis1, _ridge, twist_transform in sym.chiral:orbit(a1, ridge, t) do
        self.twists:add(axis1, twist_transform, {
          name = names.set(t:transform(a2), t:transform(a3)),
          gizmo_pole_distance = 1 * gizmo_size,
        })
      end

      local edge = ridge + a4.vector -- edge orthogonal to `a1`
      local t = sym:thru(3, 2)
      for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, t) do
        self.twists:add(axis1, twist_transform, {
          name = names.set(t:transform(a2) .. t:transform(a3) .. t:transform(a4)),
          gizmo_pole_distance = 2/sqrt(3) * gizmo_size,
        })
      end
    end,

    tags = { 'experimental' },
  }
end

def_simplex('A', {2/3}) -- simplex A = edges, no ridges
def_simplex('B', {1/4}) -- simplex B = edges & ridges, but no centers
def_simplex('C', {0}) -- simplex C = edges, ridges, & centers
def_simplex('pyra', {7/3, 2/3}) -- simplex C = edges, ridges, & centers

-- nth-order pyraminx lookalike, math by Milo
-- depths = -1 (a/n) + 4(1 - a/n) for n = layers, a:1..(n-1)
-- (can also replace 4 with NDIM to get general formula)
