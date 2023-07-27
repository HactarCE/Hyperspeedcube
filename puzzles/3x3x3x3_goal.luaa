--version 0

hypercube = shape{
    name = 'hypercube',
    ndim = 4,
    symmetry = schlafli(4, 3, 3),
    facets = {{
        pole = X,
        names = { 'Right', 'Left', 'Down', 'Up', 'Front', 'Back', 'Out', 'In' },
    }},
    facet_order = {
        'Right', 'Left',
        'Up',    'Down',
        'Front', 'Back',
        'Out',   'In',
    },
    facet_colors = {
      Right = colors.red,   Left = colors.orange,
      Up    = colors.white, Down = colors.yellow,
      Front = colors.green, Back = colors.blue,
      Out   = colors.pink,  In   = colors.purple,
    },
}

hypercubic = twists{
    name = 'hypercubic',
    symmetry = schlafli(4, 3, 3),
    axes = {
        {
            normal = X,
            cuts = range{ -1, 1, n=2 },
            names = { 'R', 'L', 'D', 'U', 'F', 'B', 'O', 'I' }
        },
        display_order = { 'R', 'L', 'U', 'D', 'F', 'B', 'O', 'I' },
    },
    generate = function(p)
        sym = schlafli(4, 3, 3)

        -- Define twists
        for rf in sym.expand do
            local I = rf * p.axes.I

            local F = rf * p.axes.F
            local U = rf * p.axes.U
            local R = rf * p.axes.R
            F, U, R = table.unpack(p.axes.canonicalize(p.axes.display_order, { F, U, R }))

            p:twist(I .. F, rot{ order = 4, bivector = ~(I ^ F) })
            p:twist(I .. F .. U, rot{ order = 2, qtm = 3, bivector = ~(I ^ (F + U)) })
            p:twist(I .. F .. U .. R, rot{ order = 3, qtm = 2, bivector = ~(I ^ (F + U + R)) })
        end

        -- Define twist directions
        p:twist_direction('x', { R = 'RO', L = 'LI', default = function(a) a .. 'R' })
        p:twist_direction('y', { U = 'UO', D = 'DI', default = function(a) a .. 'U' })
        p:twist_direction('z', { F = 'FO', B = 'BI', default = function(a) a .. 'F' })
        p:combine_twist_directions('x', 'y', 'z')

        p:alias('M', { axis = 'L', layers = '{2..-2}' })
        p:alias('E', { axis = 'D', layers = '{2..-2}' })
        p:alias('S', { axis = 'F', layers = '{2..-2}' })
        p:alias('P', { axis = 'O', layers = '{2..-2}' })

        -- Define rotations
        local axes = { x = X, y = Y, z = Z, w = W }
        for a1, v1 in ipairs(axes) do
            for a2, v2 in ipairs(axes) do
                p:rotation(a1 .. a2, rot(v1, v2))
            end
        end

        -- Define piece types
        local I = rf * p.axes.I
        local R = rf * p.axes.R
        local U = rf * p.axes.U
        local F = rf * p.axes.F
        p:set_type('core',   sym.orbit(I[2] & R[2] & U[2] & F[2]))
        p:set_type('center', sym.orbit(I[1] & R[2] & U[2] & F[2]))
        p:set_type('ridge',  sym.orbit(I[1] & R[1] & U[2] & F[2]))
        p:set_type('edge',   sym.orbit(I[1] & R[1] & U[1] & F[2]))
        p:set_type('corner', sym.orbit(I[1] & R[1] & U[1] & F[1]))
    end,
}

define_puzzle {
    name = '3x3x3x3',
    inventor = '????',
    shape = shapes.hypercube,
    twists = twists.hypercubic,
}
