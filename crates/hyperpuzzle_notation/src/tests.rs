use pretty_assertions::assert_eq;
use proptest::prelude::*;

use crate::*;

#[test]
fn test_bracketed_transform() {
    let cfg = Features::default();

    let expected = NodeList(vec![
        RepeatableNode::Move {
            layer_mask: LayerMask::default(),
            family: "R".into(),
            transform: Some("F -> U".into()),
        }
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
                RepeatableNode::Move {
                    layer_mask: LayerMask {
                        invert: true,
                        contents: Some(LayerMaskContents::Single(2)),
                    },
                    family: "aC".into(),
                    transform: None,
                }
                .with_multiplier(16),
                RepeatableNode::Rotation {
                    family: Str::default(),
                    transform: None,
                }
                .with_multiplier(1),
                RepeatableNode::BinaryGroup {
                    kind: BinaryGroupKind::Commutator,
                    contents: [
                        NodeList(vec![
                            RepeatableNode::Rotation {
                                family: "yx".into(),
                                transform: None,
                            }
                            .with_multiplier(-1),
                        ]),
                        NodeList(vec![
                            RepeatableNode::Rotation {
                                family: "U".into(),
                                transform: Some("1 j".into()),
                            }
                            .with_multiplier(-1),
                            RepeatableNode::Move {
                                layer_mask: LayerMask {
                                    invert: true,
                                    contents: None,
                                },
                                family: "IUR".into(),
                                transform: None,
                            }
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
        use crate::charsets::*;

        let family = FAMILY_REGEX.prop_map_into();
        let opt_family = prop_oneof![Just(String::new()), FAMILY_REGEX.boxed()].prop_map_into();
        let opt_transform = prop_oneof![
            Just(None),
            "[A-Z]([A-Z ]*[A-Z])?".prop_map_into().prop_map(Some)
        ];

        let leaf_repeatable_node = prop_oneof![
            (LayerMask::arbitrary(), family, opt_transform.clone()).prop_map(
                |(layer_mask, family, transform)| RepeatableNode::Move {
                    layer_mask,
                    family,
                    transform,
                }
            ),
            (opt_family, opt_transform)
                .prop_map(|(family, transform)| RepeatableNode::Rotation { family, transform }),
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
