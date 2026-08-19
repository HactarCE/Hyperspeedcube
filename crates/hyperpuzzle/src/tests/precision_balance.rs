use hyperpuzzle_core::{Catalog, Puzzle};

use super::load_new_catalog;

// # 600-Cell peice count derivation
//
// ## Shallow Icosahedron (Radio 1.5)
//
// - core: 1
// -  F  : 1
// -   E : 1
// -    V: 1
// -  F V: 1
// -   EV: 1
//
// ## Astral Icosahedron (Radio 14) = Anti-Astral Icosahedron
//
// - core: 1 (or anticore for anti-astral)
// -  F  : 3
// -   E : 3
// -   EV: 4
// -  F V: 3
// -  FEV: 2
// -    V: 2
//
// ## Half-Cut Icosahedron (Radio 15)
//
// -  F  : 1
// -   EV: 1
// -    V: 1
//
// ## 600-Cell
//
// ### Without vertex: Shallow Icosahedron
//
// - core: 1 * (1 coset)
// - C   : 1 * (600 cosets)
// -  F  : 1 * (1200 cosets)
// -   E : 1 * (720 cosets)
// - C E : 1 * (600*6 = 3600 cosets)
// -  FE : 1 * (1200*3 = 3600 cosets)
//
// ### With vertex: (Astral + Anti-Astral - Half-Cut - Shallow) Icosahedron
//
//    V: 1 * (120 cosets)
// C  V: 4 * (600*4 = 2400 cosets)
//  F V: 5 * (1200*3 = 3600 cosets)
//  FEV: 6 * (1200*3*2 = 7200 cosets)
// C EV: 5 * (600*6*2 = 7200 cosets)
// CFEV: 4 * (600*4*3*2 = 14400 cosets)
//   EV: 2 * (720*2 = 1440 cosets)
//
// ### Total
//
// - Including core: 177,121 pieces
// - Excluding core: 177,120 pieces

#[test]
pub fn test_approx_precision() {
    let catalog = load_new_catalog();

    for (id, expected_color_count, expected_piece_count) in [
        // Check that the precision is generous enough
        ("ngon_ft_shallow(200,3)", 200, 400),
        // Check that precision is strict enough
        ("rhombicuboctahedron_14", 26, 386),
        ("rhombicuboctahedron_21", 26, 554),
        ("rhombicuboctahedron_25", 26, 578),
        ("600cell_ft_shallow", 600, 177120),
    ] {
        println!("Building {id} ...");
        let puz = catalog
            .build_blocking::<Puzzle>(&id.parse().unwrap())
            .unwrap();
        println!("Checking {id} ...");
        assert_eq!(puz.colors.len(), expected_color_count);
        let piece_count = puz
            .pieces
            .iter_values()
            .filter(|p| !p.stickers.is_empty()) // skip internals
            .count();
        assert_eq!(piece_count, expected_piece_count);
    }
}

#[test]
fn test_polygon_precision_limit() {
    let catalog = load_new_catalog();

    // Increase until it breaks
    let mut n = 4;
    while is_ngon_ft_shallow_ok(&catalog, n) {
        n *= 2;
    }
    let mut max_good = n / 2;
    let mut min_bad = n;
    while max_good + 1 < min_bad {
        let test = (max_good + min_bad) / 2;
        if is_ngon_ft_shallow_ok(&catalog, test) {
            max_good = test;
        } else {
            min_bad = test;
        }
    }
    println!("Done! First bad polygon is {min_bad}");
    assert!(min_bad >= 4000, "regression in max polygon");
}

fn is_ngon_ft_shallow_ok(catalog: &Catalog, n: usize) -> bool {
    let id = format!("ngon_ft_shallow({n},3)");
    println!("Building {id} ...");
    let result = catalog.build_blocking::<Puzzle>(&id.parse().unwrap());
    match result {
        Ok(puz) => {
            let piece_count = puz
                .pieces
                .iter_values()
                .filter(|p| !p.stickers.is_empty())
                .count();
            let expected = n * 2;
            if piece_count == expected {
                println!("{id} is ok");
                true
            } else {
                println!("{id} is bad (expected {expected} pieces, got {piece_count})");
                false
            }
        }
        Err(e) => {
            println!("{id} is bad ({e})");
            false
        }
    }
}
