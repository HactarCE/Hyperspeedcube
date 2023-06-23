--version 0

define_puzzle {
    name = '3x3x3',
    shape = shape{
        ndim = 3,
        symmetry = schlafli(4, 3),
        facets = { pole = X },
    },
    twists = twists{
        symmetry = schlafli(4, 3),
        axes = { normal = X, cuts = range{ -1, 1, n=2 } },
        generate = function(p)
            for ax in p.axes do p:twist{ axis = ax, order = 4 } end
        end,
    },
}
