local R = self.axes.R(1)
local U = self.axes.R(2)
local F = self.axes.R(3)
local RF = self.twists['U+'].transform:transform(R)
local UF = self.twists['R+'].transform:transform(F)
local RU = self.twists['F+'].transform:transform(U)



local function all_but(axis)
  local ret = region()
end

symmetry(self.twists.U.transform):orbit(self.twists.R)





self.piece_types = {
  {
    name = "centers",
    { name = "T-centers" },
  },
  "ridges", {

  },
  "edges", {
    "edges",
    "wings", {

    }
  },
  "corners",
}

self.piece_types = {
  {
    name = "centers",
    has = U(1),
    missing = {R(1), RF(1)},
    symmetry = 'bc3',
  },
  {
    name = "corners",
    subtypes = {
      {
        name = "corner orbit A",
        has = ...,
      },
      {
        name = "corner orbit B",
        has = ...,
        missing = ...,
        blocks = ...,
        subtypes = {
          ...
        }
      },
    }
  }
}
