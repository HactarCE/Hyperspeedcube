--version 0

cube = shape{
    name = 'cube',
    ndim = 3,
    symmetry = schlafli(4, 3),
    facets = {
        pole = X,
        names = { 'Right', 'Left', 'Down', 'Up', 'Front', 'Back' }
    },
    facet_order = {
        'Right', 'Left',
        'Up',    'Down',
        'Front', 'Back',
    },
    facet_colors = {
        Right = colors.red,   Left = colors.orange,
        Up    = colors.white, Down = colors.yellow,
        Front = colors.green, Back = colors.blue,
    },
}

cubic = twists{
    name = 'cubic',
    symmetry = schlafli(4, 3),
    axes = {
        {
            normal = X,
            cuts = range{ -1, 1, n=2 },
            names = { 'R', 'L', 'D', 'U', 'F', 'B' },
        },
        display_order = { 'R', 'L', 'U', 'D', 'F', 'B' },
    },
    generate = function(p)
        sym = schlafli(4, 3)

        -- Define twists
        for ax in p.axes do p:twist{ axis = ax, order = 4 } end

        -- Define twist directions
        p:twist_direction('CW', function(a) a end)
        p:twist_direction('CCW', function(a) a .. "'" end)
        p:twist_direction('180 CW', function(a) a .. "2" end)
        p:twist_direction('180 CCW', function(a) a .. "2'" end)

        p:alias('M', { axis = 'L', layers = '{2..-2}' })
        p:alias('E', { axis = 'D', layers = '{2..-2}' })
        p:alias('S', { axis = 'F', layers = '{2..-2}' })
        p:wide_suffix('w')

        -- Define rotations
        p:twist_direction('x', rot{ bivector = ~X, order = 4 })
        p:twist_direction('y', rot{ bivector = ~Y, order = 4 })
        p:twist_direction('z', rot{ bivector = ~Z, order = 4 })

        -- Define piece types
        local R = rf * p.axes.R
        local U = rf * p.axes.U
        local F = rf * p.axes.F
        p:set_type('core', sym.orbit(R[2] & U[2] & F[2]))
        p:set_type('center', sym.orbit(R[1] & U[2] & F[2]))
        p:set_type('edge', sym.orbit(R[1] & U[1] & F[2]))
        p:set_type('corner', sym.orbit(R[1] & U[1] & F[1]))
    end,
}

define_puzzle {
    name = '3x3x3',
    inventor = 'Ernő Rubik',
    shape = shapes.cube,
    twists = twists.cubic,
}
