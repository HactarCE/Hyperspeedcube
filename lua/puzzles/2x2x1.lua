local utils = lib.utils

VERSION = '0.1.0'
TAGS = {
  author = { "Luna Harran" },
}

puzzles:add({
    id = '2x2x1',
    version = '0.1.0',
    name = "2x2x1",
    ndim = 3,
    build = function(self)
        local sym = cd{4,2}
        local a = sym.oxo
        local b = sym.oox

        self:carve(sym:orbit(a))
        self:carve(sym:orbit(b))

        -- Define axes and slices
        self.axes:add(sym:orbit(a), {INF, 0, -INF})

        for _, axis, twist_transform in sym.chiral:orbit(self.axes[a.unit], sym:thru(1, 3)) do
            self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
        end
    end
})
