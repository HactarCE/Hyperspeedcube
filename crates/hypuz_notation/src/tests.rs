use pretty_assertions::assert_eq;
use proptest::prelude::*;

use crate::*;

#[test]
fn test_bracketed_transform() {
    let cfg = Features::default();

    let expected = NodeList(vec![
        RepeatableNode::Move(Move {
            layers: LayerPrefix::default(),
            rot: Rotation {
                family: "R".into(),
                transform: Some("F -> U".into()),
            },
        })
        .with_multiplier(1),
    ]);
    assert_eq!(expected, parse_notation("R[ F -> U ]", cfg).unwrap());
    assert_eq!(expected, parse_notation("R[F -> U]", cfg).unwrap());
    assert_eq!(expected.to_string(), "R[F -> U]");
}

#[test]
fn test_nested_notation() {
    let cfg = Features::default();

    let expected = NodeList(vec![
        RepeatableNode::Group {
            kind: GroupKind::Simple,
            contents: NodeList(vec![
                RepeatableNode::Move(Move {
                    layers: LayerPrefix {
                        invert: true,
                        contents: Some(LayerPrefixContents::Single(Layer::new(2).unwrap())),
                    },
                    rot: Rotation {
                        family: "aC".into(),
                        transform: None,
                    },
                })
                .with_multiplier(16),
                RepeatableNode::Rotation(Rotation {
                    family: Str::default(),
                    transform: None,
                })
                .with_multiplier(1),
                RepeatableNode::BinaryGroup {
                    kind: BinaryGroupKind::Commutator,
                    contents: [
                        NodeList(vec![
                            RepeatableNode::Rotation(Rotation {
                                family: "yx".into(),
                                transform: None,
                            })
                            .with_multiplier(-1),
                        ]),
                        NodeList(vec![
                            RepeatableNode::Rotation(Rotation {
                                family: "U".into(),
                                transform: Some("1 j".into()),
                            })
                            .with_multiplier(-1),
                            RepeatableNode::Move(Move {
                                layers: LayerPrefix {
                                    invert: true,
                                    contents: None,
                                },
                                rot: Rotation {
                                    family: "IUR".into(),
                                    transform: None,
                                },
                            })
                            .with_multiplier(16),
                        ]),
                    ],
                }
                .with_multiplier(-42),
            ]),
        }
        .with_multiplier(1),
    ]);

    assert_eq!(
        expected.to_string(),
        "(~2aC16 @ [@yx', @U[1 j]' ~IUR16]42')",
    );

    // with normal spaces
    assert_eq!(
        expected,
        parse_notation("(~2aC16 @ [@yx', @U[1 j]' ~IUR16]42')", cfg).unwrap()
    );

    // with minimal spaces
    assert_eq!(
        expected,
        parse_notation("(~2aC16 @ [@yx',@U[1 j]' ~IUR16]42')", cfg,).unwrap()
    );

    // with extra spaces
    assert_eq!(
        expected,
        parse_notation(
            "  (  ~2aC16  @  [  @yx'  ,  @U[1 j]'  ~IUR16  ]42'  )  ",
            cfg,
        )
        .unwrap()
    );
}

#[test]
fn test_notation_errors() {
    let cfg = Features::MAXIMAL;

    parse_notation("R [F -> U]", cfg).expect_err("space between family and bracketed transform");
    parse_notation("-2R", cfg).expect_err("negative simple layer");
    parse_notation("{-2-4}R", cfg).expect_err("negative HSC1 layer range");

    // spaces in bad places
    parse_notation("(~2 aC16 @ [@yx',@U[1 j]' ~IUR16]42')", cfg).expect_err("space after tilde");
    parse_notation("(~2 aC16 @ [@yx',@U[1 j]' ~IUR16]42')", cfg).expect_err("space after layer");
    parse_notation("(~2aC16 @ [@yx ',@U[1 j]' ~IUR16]42')", cfg).expect_err("lone multiplier");
    parse_notation("(~2aC16 @ [@yx',@U[1 j]' ~IUR16] 42')", cfg)
        .expect_err("space before multiplier");
    parse_notation("(~2aC16 @ [@yx',@U[1 j]' ~IUR16]4 2')", cfg).expect_err("lone multiplier");

    // missing spaces
    parse_notation("(~2aC16@ [@yx',@U[1 j]' ~IUR16]42')", cfg).expect_err("no space before @");
    parse_notation("(~2aC16 @[@yx',@U[1 j]' ~IUR16]42')", cfg)
        .expect_err("brackets inside rotation transform");

    parse_notation("(~2aC16 @ [@yx',,@U[1 j]' ~IUR16]42')", cfg).expect_err("extra comma");
    parse_notation("(~2aC16 @ [@yx',@U[1 j]',~IUR16]42')", cfg).expect_err("extra comma");
}

impl Arbitrary for Node {
    type Parameters = ();

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        let leaf_repeatable_node = prop_oneof![
            Move::arbitrary().prop_map(RepeatableNode::from),
            Rotation::arbitrary().prop_map(RepeatableNode::from),
        ];
        let leaf_node = prop_oneof![
            (leaf_repeatable_node, Multiplier::arbitrary())
                .prop_map(|(rep_node, mult)| rep_node.with_multiplier(mult)),
            Just(Node::Pause),
            Sq1Move::arbitrary().prop_map_into(),
            MegaminxScrambleMove::arbitrary().prop_map_into(),
        ];

        leaf_node
            .prop_recursive(
                3,  // 3 levels deep
                20, // max size of 20 nodes
                4,  // 4 items per collection
                |inner| {
                    let node_list = prop::collection::vec(inner.clone(), 0..10).prop_map(NodeList);
                    let branch_repeatable_node = prop_oneof![
                        (GroupKind::arbitrary(), node_list.clone())
                            .prop_map(|(kind, contents)| RepeatableNode::Group { kind, contents }),
                        (BinaryGroupKind::arbitrary(), [node_list.clone(), node_list]).prop_map(
                            |(kind, contents)| RepeatableNode::BinaryGroup { kind, contents }
                        ),
                    ];
                    let branch_node = (branch_repeatable_node, Multiplier::arbitrary())
                        .prop_map(|(rep_node, mult)| rep_node.with_multiplier(mult));
                    branch_node
                },
            )
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for Move {
    type Parameters = ();

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use crate::charsets::FAMILY_REGEX;

        let layers = LayerPrefix::arbitrary();
        let family = FAMILY_REGEX.prop_map_into();
        let opt_transform = prop_oneof![
            Just(None),
            "[A-Z]([A-Z ]*[A-Z])?".prop_map_into().prop_map(Some)
        ];

        (layers, family, opt_transform)
            .prop_map(|(layers, family, transform)| Move {
                layers,
                rot: Rotation { family, transform },
            })
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for Rotation {
    type Parameters = ();

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use crate::charsets::FAMILY_REGEX;

        let opt_family =
            prop_oneof![Just(String::new()), FAMILY_REGEX.prop_map_into().boxed()].prop_map_into();
        let opt_transform = prop_oneof![
            Just(None),
            "[A-Z]([A-Z ]*[A-Z])?".prop_map_into().prop_map(Some)
        ];

        (opt_family, opt_transform)
            .prop_map(|(family, transform)| Rotation { family, transform })
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

proptest! {
    #[test]
    fn proptest_notation_roundtrip(node_list: NodeList) {
        assert_notation_roundtrip(node_list);
    }
}

fn assert_notation_roundtrip(node_list: NodeList) {
    let features = Features::MAXIMAL;
    assert_eq!(
        parse_notation(&node_list.to_string(), features),
        Ok(node_list),
    );
}

#[test]
fn test_resolve_signed_layer() {
    assert_eq!(None, resolve_signed_layer(3, -4));
    assert_eq!(Some(1), resolve_signed_layer(3, -3));
    assert_eq!(Some(2), resolve_signed_layer(3, -2));
    assert_eq!(Some(3), resolve_signed_layer(3, -1));
    assert_eq!(Some(1), resolve_signed_layer(3, 1));
    assert_eq!(Some(2), resolve_signed_layer(3, 2));
    assert_eq!(Some(3), resolve_signed_layer(3, 3));
    assert_eq!(None, resolve_signed_layer(3, 4));
}

proptest! {
    #[test]
    fn proptest_resolve_signed_layer_no_panic(signed_layer: SignedLayer, layer_count: u16) {
        signed_layer.resolve(layer_count); // don't panic!
    }

    #[test]
    fn proptest_resolve_signed_layer_range_no_panic(range: [SignedLayer; 2], layer_count: u16) {
        SignedLayer::resolve_range(range, layer_count); // don't panic!
    }

    #[test]
    fn proptest_resolve_signed_layer_range_correctness(
        layer_count in 1..=5_u16,
        lo in -10..=10_i16,
        hi in -10..=10_i16,
    ) {
        prop_assume!(lo != 0 && hi != 0);
        let lo = SignedLayer::new(lo).unwrap();
        let hi = SignedLayer::new(hi).unwrap();
        let actual: Vec<Layer> = SignedLayer::resolve_range([lo, hi], layer_count)
            .map(|range| range.into_iter().collect())
            .unwrap_or_default();
        let mut lo = lo.to_i16();
        if lo < 0 {
            lo = layer_count as i16 + lo + 1;
        }
        let mut hi = hi.to_i16();
        if hi < 0 {
            hi = layer_count as i16 + hi + 1;
        }
        prop_assume!(lo <= hi);
        let expected: Vec<Layer> = (lo..=hi)
            .filter(|x| (1..=layer_count as i16).contains(x))
            .map(|i| Layer::new(i as u16).unwrap())
            .collect();
        assert_eq!(expected, actual);
    }
}

#[test]
fn test_layer_prefix_contents_to_ranges() {
    fn to_ranges(contents: LayerPrefixContents, layer_count: u16) -> Vec<[u16; 2]> {
        contents
            .to_ranges(layer_count)
            .into_iter()
            .map(|r| [r.start().to_u16(), r.end().to_u16()])
            .collect()
    }

    let single = |i| LayerPrefixContents::Single(Layer::new(i).unwrap());

    let range = |i, j| {
        LayerPrefixContents::Range(LayerRange::new(
            Layer::new(i).unwrap(),
            Layer::new(j).unwrap(),
        ))
    };

    let empty: Vec<[u16; 2]> = vec![];

    assert_eq!(to_ranges(single(1), 5), vec![[1, 1]]);
    assert_eq!(to_ranges(single(2), 5), vec![[2, 2]]);
    assert_eq!(to_ranges(single(5), 5), vec![[5, 5]]);
    assert_eq!(to_ranges(single(6), 5), empty);

    assert_eq!(to_ranges(range(1, 3), 5), vec![[1, 3]]);
    assert_eq!(to_ranges(range(2, 4), 5), vec![[2, 4]]);
    assert_eq!(to_ranges(range(2, 10), 5), vec![[2, 5]]);
    assert_eq!(to_ranges(range(5, 10), 5), vec![[5, 5]]);
    assert_eq!(to_ranges(range(6, 10), 5), empty);
}

fn resolve_signed_layer(layer_count: u16, layer: i16) -> Option<u16> {
    SignedLayer::new(layer)
        .unwrap()
        .resolve(layer_count)
        .map(|l| l.to_u16())
}
