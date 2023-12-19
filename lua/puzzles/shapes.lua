common = require('common')

local function def_platonic_solid(name, sym)
  puzzledef{
    id = name:lower(),
    name = name,
    ndim = #sym + 1,
    build = function()
      local s = ''
      for i = 1,#sym do
        s = s .. 'o'
      end
      local seed = s .. 'x'
      for v in cd(sym):expand(seed) do
        carve(v)
        add_color(v)
      end
    end,
  }
end

def_platonic_solid("Simplex", {3, 3, 3})
def_platonic_solid("Hypercube", {4, 3, 3})
def_platonic_solid("16-cell", {3, 3, 4})
def_platonic_solid("24-cell", {3, 4, 3})
def_platonic_solid("120-cell", {5, 3, 3})
def_platonic_solid("600-cell", {3, 3, 5})

def_platonic_solid("Tetrahedron", {3, 3})

def_platonic_solid("5-cube", {4, 3, 3, 3})
def_platonic_solid("5-simplex", {3, 3, 3, 3})
def_platonic_solid("5-orthoplex", {3, 3, 3, 4})
