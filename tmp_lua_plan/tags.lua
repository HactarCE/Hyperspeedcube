addtag('cubic')

tagsetdef{
  name = 'Dimension',
  prefix = 'dim/',
  members = {'2d', '3d', '4d', '5d', '6d', '7d', '8d'},
  open = false,
  auto = function(puzzle)
    return 'dim/' .. puzzle.space.ndim .. 'd'
  end,
}

tagsetdef{
  name = 'Rank',
  prefix = 'rank/',
  members = {'2', '3', '4', '5', '6', '7', '8', 'unknown'},
  open = false,
  auto = function(puzzle)
    return 'unknown'
  end,
}

tagsetdef{
  name = 'Type',
  members = {'solid', 'tiling', 'soup'},
  open = false,
  auto = function(puzzle)
    return 'soup' -- unsure
  end,
}

tagsetdef{
  name = 'Shape',
  open = true,
  default = 'other',
}

tagsetdef{
  name = 'Turns',
  open = true,
}

tagsetdef{
  name = 'Axes',
  open = true,
}

tagsetdef{
  name = 'Cut depth',
  members = {
    'shallow cut',
    'cut to adjacent',
    'deep cut',
    'deeper than adjacent',
    'deeper than origin',
  },
}

tagsetdef{
  name = 'Turning properties',
  members = {
    'doctrinaire',
    'bandaged',
    'unbandaged',
    'shapeshifting',
    'jumbling',
  },
  open = true,
}

addtag{'reduced', 'sliding', 'circle', category = 'Turning properties'}



addtag{category = 'shape', open = true}
addtag{category = ''}
