for k, v in pairs(require('utils/*')) do
  _G[k] = v
end

function lerp(a, b, t)
  return a + (b-a)*t
end

function nth_uppercase_name(n)
  local ret = ''
  while n > 0 do
    n = n - 1
    ret = string.char(string.byte('A') + (n%26)) .. ret
    n = floor(n / 26)
  end
  return ret
end

function uppercase_name_to_n(name)
  local ret = 0
  for i = 1, #name do
    ret = ret * 26 + string.byte(name:sub(i, i)) - string.byte('A') + 1
  end
  return ret
end

function cut_shape(puzzle, shape, cut_depths, ...)
  local poles = shape:iter_poles(...)
  local colors = puzzle:carve(poles)
  local axes = cut_depths and puzzle.axes:add(poles, cut_depths)
  return colors, axes
end

function add_puzzle_twists(puzzle_recipe)
  local puzzle = puzzle_recipe.puzzle

  for _, twist_set in ipairs(puzzle_recipe.twist_sets) do
    for i, refl1 in ipairs(twist_set.reflections) do
      local g1, unfix1 = table.unpack(refl1)
      assert(g1.is_refl) -- TODO: handle if `g1` is a rotation
      for j, refl2 in ipairs(twist_set.reflections) do
        local g2, unfix2 = table.unpack(refl2)
        assert(g2.is_refl)
        if i >= j then goto continue end

        local twist_transform = g2 * g1

        local fix = twist_set.fix - unfix1 - unfix2

        for t in twist_set.symmetry:orbit(fix) do
          if puzzle.ndim == 3 then
            puzzle.twists:add(t:transform(twist_set.axis), t:transform_oriented(twist_transform), { gizmo_pole_distance = 1 })
          elseif puzzle.ndim == 4 then
            error('todo')
          else
            error("can't do other dimensions")
          end
        end


        ::continue::
      end
    end
  end
end

-- Concatenates the sequences.
function concatseq(...)
  local ret = {}
  for _, t in ipairs({...}) do
    for _, v in ipairs(t) do
      table.insert(ret, v)
    end
  end
  return ret
end
