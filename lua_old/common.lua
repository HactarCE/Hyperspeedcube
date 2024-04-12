axes = require('common/axes')
colors = require('common/colors')

function carve_and_slice_face_turning(sym, depth, init_vector)
  init_vector = init_vector or 'oox'
  for v in cd(sym):expand(init_vector) do
    carve(v)
    slice{ normal = v, distance = v:mag() * depth }
  end
end

function multiplier_suffix(m)
  assert(type(m) == 'number')

  if m < 0 then
	return multiplier_suffix(-m) .. "'"
  elseif m == 1 then
    return ""
  else
    return tostring(m)
  end
end

function twist_directions_2d(order)
  assert(type(order) == 'number')

  local ret = {
    CW = function(ax) twist(ax) end,
    CCW = function(ax) twist(ax .. "'") end,
  }

  local max_multiplier = order // 2
  for i = 2, max_multiplier do
    local angle = round(i * 360 / order)
    ret[angle .. " CW"] = function(ax) twist(ax .. i) end
    ret[angle .. " CCW"] = function(ax) twist(ax .. i .. "'") end
  end

  return ret
end

function symmetric_twists_3d(sym, depths, ax, from, to)
  define_twists{
    axis = ax,
    depths = depths,
    generator = rot{plane = axes(ax), from = axes(from), to = axes(to)},
    multipliers = 'auto',
    symmetry = sym,
    name = function(t, m)
      -- t = group element that got us to this twist
      -- m = multiplier of twist (integer)
      return twist(t * axes(ax) .. common.multiplier_suffix(m))
    end,
  }
end
