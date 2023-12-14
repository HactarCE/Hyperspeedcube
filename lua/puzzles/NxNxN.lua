common = require('common')

function puzzledef_NxNxN(args)
  local layer_count = args.layer_count
  local aliases = args.aliases
  local meta = args.meta

  table.append(aliases, "{4, 3} " .. layer_count)
  table.append(aliases, layer_count .. "^3")
  puzzledef{
    name = layer_count .. "x" .. layer_count .. "x" .. layer_count,
    aliases = aliases,
    ndim = 3,
    meta = meta,
    properties = {
      shallow_cut = true,
      doctrinaire = true,
    },
    build = function()
      local cuts = {}
      for i = 1, layer_count - 1 do
        cuts.insert((i / layer_count) * 2 - 1)
      end

      fold{4, 3}
      carve(svec('z'))
      for _, cut in ipairs(cuts) do
        slice(svec('z') * cut)
      end
      unfold()

      define_facets(common.facets.cube())
      define_axes(common.axes.cubic(cuts))

      R, U, F = axis'R', axis'U', axis'F'

      define_twists(common.symmetric_twists_3d({4, 3}, F, U, R))
      define_twist_directions(common.twist_directions_2d(4))

      define_piece_types{
        symmetry = {4, 3},
        corners = R(1) & U(1) & F(1),
        edges   = R(1) & U(1),
        centers = R(1),
      }
    end,
  }
end

-- puzzledef{
--   name = "3x3x3",
--   aliases = {
--     "{4, 3} 3",
--     "3^3",
--     "Rubik's Cube",
--   },
--   ndim = 3,
--   meta = {
--     id = '3x3x3',
--     author = "Andrew Farkas",

--     year = 1970,
--     inventor = "Ernő Rubik",

--     family = "wca",
--     external = {
--       pcubes = "3x3x3",
--       gelatinbrain = "3.1.2",
--       museum = 2968,
--     },
--   },

--   properties = {
--     shallow_cut = true,
--     doctrinaire = true,
--   },

--   build = function()
--     fold{4, 3}
--     carve(svec('z'))
--     slice(svec('z') * 1/3)
--     unfold()

--     define_facets(common.facets.cube())
--     define_axes(common.axes.cubic{1/3, -1/3})

--     R, U, F = axis'R', axis'U', axis'F'

--     define_twists(common.symmetric_twists_3d({4, 3}, F, U, R))
--     define_twist_directions(common.twist_directions_2d(4))

--     define_piece_types{
--       symmetry = {4, 3},
--       corners = R(1) & U(1) & F(1),
--       edges   = R(1) & U(1),
--       centers = R(1),
--     }
--   end,
-- }


-- function standard_piece_types_3d(sym, R, U, F, layer_count)

--   Self::Piece => format!("piece"),
--   Self::Corner => format!("corner"),
--   Self::Edge => format!("edge"),
--   Self::Wing(0) => format!("wing"),
--   Self::Wing(x) => format!("wing ({x})"),
--   Self::Center => format!("center"),
--   Self::TCenter(0) => format!("T-center"),
--   Self::TCenter(x) => format!("T-center ({x})"),
--   Self::XCenter(0) => format!("X-center"),
--   Self::XCenter(x) => format!("X-center ({x})"),
--   Self::Oblique(0, 0) => format!("oblique"),
--   Self::Oblique(x, y) => format!("oblique ({x},{y})"),

--   local corner_type = {name = 'corner', pieces = R(1) & U(1) & F(1)}
--   local edge_type = {name = 'edge', pieces = R(1) & U(1)}
--   local center_type = {name = 'center', pieces = R(1)}

--   order = layer_count // 2

--   if layer_count >= 4 then
--     if layer_count % 2 == 1 then
--       table.insert(edge_type.subtypes, {name = 'edge', pieces = F(order + 1)})
--     end
--     if layer_count >= 6 then
--       for i = 2, order do
--         table.insert(edge_type.subtypes, {
--           name = 'wing',
--           disambiguation = order - i + 1
--           pieces = F(i),
--         })
--       end
--     end
--   end

--   if layer_count >= 4 then
--     if layer_count % 2 == 1 then
--       table.insert(center_type.subtypes, {name = 'center', pieces = U(order + 1) & F(order + 1)})
--       for i = 2, order do
--         table.insert(center_type.subtypes, {name = 'T-center', pieces = U(order + 1) & F(i)})
--       end
--     end
--     for i = 2, order do
--       table.insert(center_type.subtypes, {
--         name = 'X-center',
--         disambiguation = order - i + 1
--         pieces = U(i) & F(i),
--       })
--     end
--     for i = 2, order do
--       table.insert(center_type.subtypes)
--     end
--   end

--   return {
--     symmetry = sym,
--     corner_type,
--     edge_type,
--     center_type,
--   }
-- end
