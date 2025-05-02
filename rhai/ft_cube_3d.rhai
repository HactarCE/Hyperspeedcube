const CUBE_FACE_NAMES = #{
    F: [],
    U: [2, "F"],
    R: [1, "U"],
    L: [0, "R"],
    D: [1, "L"],
    B: [2, "D"],
};

add_color_system_generator(#{
    id: "prism",
    name: "Polygonal Prism",
    params: [
        #{ name: "Polygon size", type: "int", init: 5, min: 3, max: 24 },
    ],
    gen: |params| {
        print(params);
        let n = params[0];
        let colors = [
            #{ name: "U", display: "Up",   init: "Mono Dyad [1]" },
            #{ name: "D", display: "Down", init: "Mono Dyad [2]" },
        ];
        for i in 0..n {
            let a = to_string(i); // TODO: nth_uppercase_name
            print(a);
            colors.push(#{
                name: `F${a}`,
                display: `Face ${a}`,
                init: `Rainbow [${i}/${n}]`,
            });
        }
        #{
            name: `${n}-gonal Prism`, // TODO
            colors: colors,
        }
    },
});

fn add_ft_cube_twists() {
    let sym = cd("bc3");

    with sym {
        this.add_axis(sym.oox, global::CUBE_FACE_NAMES);
        this.add_twist(
            this.axes.F,
            rot(#{ fix: this.axes.F, from: this.axes.U, to: this.axes.R }),
            #{ gizmo_pole_distance: 1 },
        );
    }

    this.add_twist_direction("CW", |axis| this[axis.name]);
    this.add_twist_direction("CW2", |axis| this[axis.name + "2"]);
    this.add_twist_direction("CCW", |axis| this[axis.name + "'"]);
    this.add_twist_direction("CCW2", |axis| this[axis.name + "2'"]);
}

fn add_cubic_vantage_group() {
    let sym = cd("bc3");
    this.add_vantage_group(#{
        id: "cubic",
        symmetry: sym,
        refs: [[sym.oox, global::CUBE_FACE_NAMES]],
        init: ["F", "U"],
    });
}

fn add_face_centered_cubic_vantage_set() {
    let sym = cd("bc3");
    let F = sym.oox;
    let U = sym.thru(2).transform(F);
    let R = sym.thru(1).transform(U);
    let x = rot(#{ fix: U, from: R, to: F });
    let y = rot(#{ fix: R, from: F, to: U });
    let z = rot(#{ fix: F, from: U, to: R });
    this.add_vantage_set(#{
        name: "Face-centered",
        group: "cubic",
        view_offset: rot(#{ from: X, to: Z, angle: 20*PI/180 })
                   * rot(#{ from: Y, to: Z, angle: 35*PI/180 }),
        transforms: #{
            x: x, "x'": x.rev(),
            y: y, "y'": y.rev(),
            z: z, "z'": z.rev(),
        },
        axes: "*",
    });
}

add_twist_system(#{
    id: "ft_cubic",
    name: "Cubic",
    ndim: 3,
    build: || {
        this.add_ft_cube_twists();
        this.add_cubic_vantage_group();
        this.add_face_centered_cubic_vantage_set();
    },
});

// Cube
add_color_system(#{
    id: "cube",
    name: "Cube",

    colors: [
        #{ name: "R", display: "Right" },
        #{ name: "L", display: "Left"  },
        #{ name: "U", display: "Up"    },
        #{ name: "D", display: "Down"  },
        #{ name: "F", display: "Front" },
        #{ name: "B", display: "Back"  },
    ],

    schemes: [
        #{
            name: "Western",
            map: #{
                R: "Red",
                L: "Orange",
                U: "White",
                D: "Yellow",
                F: "Green",
                B: "Blue",
            },
        },
        #{
            name: "Japanese",
            map: #{
                R: "Red",
                L: "Orange",
                U: "White",
                D: "Blue",
                F: "Green",
                B: "Yellow",
            },
        },
    ],
    default_scheme: "Western",
});
