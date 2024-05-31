function twist3d(axis, twist_transform)
  return {
    axis = axis,
    transform = twist_transform,
    prefix = axis.name,
    inverse = true,
    multipliers = true,
  }
end

function map_string_keys(t, map)
  if type(t) == 'string' then
    if type(map) == 'function' then
      return map(t) or t
    else
      return map[t] or t
    end
  elseif type(t) == 'table' then
    local ret = {}
    for k, v in pairs(t) do
      ret[map_string_keys(k,map)] = v
    end
    return ret
  else
    return t
  end
end

function map_string_values(t, map)
  if type(t) == 'string' then
    if type(map) == 'function' then
      return map(t) or t
    else
      return map[t] or t
    end
  elseif type(t) == 'table' then
    local ret = {}
    for k, v in pairs(t) do
      ret[k] = map_string_values(v, map)
    end
    return ret
  else
    return t
  end
end
