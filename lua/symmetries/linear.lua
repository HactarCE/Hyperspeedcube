function line(scale, basis)
  return {
    sym = cd({}, basis),
    iter_poles = function(self, name1, name2)
      return self.sym:orbit(self.sym.x.unit * (scale or 1)):named({
        [name1] = {},
        [name2] = {1},
      })
    end,
  }
end
