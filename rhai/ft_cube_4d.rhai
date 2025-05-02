import "ft_cube_common" as common;

const HYPERCUBE_FACET_NAMES = #{
    O: [],
    F: [3, "O"],
    U: [2, "F"],
    R: [1, "U"],
    L: [0, "R"],
    D: [1, "L"],
    B: [2, "D"],
    I: [3, "B"],
};

const GIZMO_EDGE_FACTOR = 0.8;

add_puzzle(#{
    id: "ft_hypercube_3",
    version: "1.0.0",
    name: "3x3x3x3",
    aliases: ["3^4"],
    colors: "hypercube",
    twists: "ft_hypercubic",
    ndim: 4,
    build: || {
        let sym = cd("bc4");
        with sym {
            this.carve(plane(sym.ooox.unit), HYPERCUBE_FACET_NAMES);
            this.slice_layers(this.axes.R, common::ft_cube_cut_depths(this.ndim, 3));
        }
    },
});

add_color_system(#{
    id: "hypercube",
    name: "Hypercube",
    colors: [
        #{ name: "R", display: "Right", init: "Red" },
        #{ name: "L", display: "Left",  init: "Orange" },
        #{ name: "U", display: "Up",    init: "White" },
        #{ name: "D", display: "Down",  init: "Yellow" },
        #{ name: "F", display: "Front", init: "Green" },
        #{ name: "B", display: "Back",  init: "Blue" },
        #{ name: "O", display: "Out",   init: "Pink" },
        #{ name: "I", display: "In",    init: "Purple" },
    ]
});

add_twist_system(#{
    id: "ft_hypercubic",
    name: "Hypercubic",
    ndim: 4,
    build: || {
        this.add_ft_hypercube_twists();
        this.add_hypercubic_vantage_group();
        this.add_facet_centered_hypercubic_vantage_set();
    },
});

fn add_ft_hypercube_twists() {
    let sym = cd("bc4");

    with sym.chiral {
        let a1 = this.add_axis(sym.ooox, global::HYPERCUBE_FACET_NAMES);
        let a2 = sym.thru(3).transform(a1);
        let a3 = sym.thru(2).transform(a2);
        let a4 = sym.thru(1).transform(a3);


        this.add_twist(a1, sym.thru([1, 0]), #{
            name: |t, i| `${t.transform(a2)}`,
            gizmo_pole_distance: 1,
        });
        this.add_twist(a1, sym.thru([2, 0]), #{
            name: |t, i| `${t.transform(a2)}${t.transform(a3)}`,
            gizmo_pole_distance: (1 + global::GIZMO_EDGE_FACTOR) / sqrt(2),
        });
        this.add_twist(a1, sym.thru([2, 1]), #{
            name: |t, i| `{${t.transform(a2)}${t.transform(a3)}${t.transform(a4)}}`,
            gizmo_pole_distance: (1 + 2 * global::GIZMO_EDGE_FACTOR) / sqrt(3),
        });

    }
}

fn add_hypercubic_vantage_group() {
    let sym = cd("bc4");
    this.add_vantage_group(#{
        id: "hypercubic",
        symmetry: sym,
        refs: [[sym.ooox, global::HYPERCUBE_FACET_NAMES]],
        init: ["I", "F", "U"],
    });
}

fn add_facet_centered_hypercubic_vantage_set() {
    let basis = #{ x: X, y: Y, z: Z, w: W };
    let rotations = #{};
    for char1 in "xyzw" {
        let a = to_string(char1);
        for char2 in "xyzw" {
            let b = to_string(char2);
            if basis[a] != basis[b] {
                rotations[a + b] = rot(#{ from: basis[a], to: basis[b] })
            }
        }
    }

    use [F, U, R] from this.axes;
    let x = rot(#{ fix: U, from: R, to: F });
    let y = rot(#{ fix: R, from: F, to: U });
    let z = rot(#{ fix: F, from: U, to: R });

    use [O, I] from this.axes;
    this.add_vantage_set(#{
        name: "Facet-centered",
        group: "hypercubic",
        view_offset: rot(#{ from: X, to: Z, angle: 45*PI/180 })
                   * rot(#{ from: Y, to: Z, angle: 35*PI/180 }),
        transforms: rotations,
        axes: "*",
        directions: #{
            I: #{
                 x:   this.IR,  y:   this.IU,  z:   this.IF,
                "x'": this.IL, "y'": this.ID, "z'": this.IB,
                // x: x, "x'": x.rev(),
                // y: y, "y'": y.rev(),
                // z: z, "z'": z.rev(),
            },
        },
        inherit_directions: |axis_vector| {
            if axis_vector == I.vector { () }
            else if axis_vector == O.vector { refl() }
            else { rot(#{ from: axis_vector, to: I }) }
        },
    });
}
