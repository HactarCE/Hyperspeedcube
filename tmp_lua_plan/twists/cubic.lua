names = {'R', 'L', 'D', 'U', 'F', 'B'}

twistdef{
  name = 'face-turning cubic',
  ndim = 3,

  tags = {
    'turns/facet',
    'axes/cubic',
  }

  build = function(params)
    local poles = cd{4, 3}:expand(vec(0, 0, params.r))
    local axes = {}
    for i, v in ipairs(poles) do
      axes[i] = {
        name = names[i],
        region = plane{pole = v}.outside,
        twists = {{
          name = names[i],
          transform = rot{axis = v, angle = tau/4},
          repeats = {2, -1}
        }},
      }
    end
    return axes
  end,

  directions = {
    ['CW'] = function(axis)
      return self.gettwist(axis.name)
    end,
    ['CCW'] = function(axis)
      return self.gettwist(axis.name + '2')
    end,
    ['180'] = function(axis)
      return self.gettwist(axis.name + "'")
    end,
  }
}
