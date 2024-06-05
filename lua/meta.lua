-- Common metatables

local function transform_table(tab, transform)
  local ret = {}
  if type(tab) == 'table' then
    for k, v in pairs(tab) do
      ret[k] = transform_table(v)
    end
  elseif type(tab) == 'string' or type(tab) == 'function' then
    ret[k] = v
  else
    ret[k] = transform:transform(tab)
  end
  return ret
end

-- Metatable for shapes that have a single
-- symmetry at key `sym` and can be transformed
shape = {
  __index = function(self, k) return self.sym[k] end,
  __transform = transform_table,
}
