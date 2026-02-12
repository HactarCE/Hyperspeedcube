use std::str::FromStr;

use chumsky::prelude::*;

use crate::spanned::*;
use crate::{Features, GroupKind, LayerFeatures, Span};

/// Error produced while parsing puzzle notation.
///
/// Multiple errors may be produced during the same parse.
pub type ParseError<'src> = Rich<'src, char, SimpleSpan>;
type ParseState = extra::SimpleState<()>;
type ParseExtra<'src> = extra::Full<ParseError<'src>, ParseState, Features>;

/// Trait alias for parser.
pub(crate) trait NotationParser<'src, O>:
    Clone + Parser<'src, &'src str, O, ParseExtra<'src>>
{
}
impl<'src, O, T> NotationParser<'src, O> for T where
    T: Clone + Parser<'src, &'src str, O, ParseExtra<'src>>
{
}

pub(crate) fn node_list_with_features<'src>(
    features: Features,
) -> impl NotationParser<'src, NodeList> {
    node_list().with_ctx(features)
}

pub(crate) fn layer_prefix_with_features<'src>(
    features: LayerFeatures,
) -> impl NotationParser<'src, LayerPrefix> {
    let features = Features {
        layers: features,
        ..Default::default()
    };
    layer_prefix().with_ctx(features)
}

fn node_list<'src>() -> impl NotationParser<'src, NodeList> {
    recursive(|node_list| {
        let commutator_or_conjugate = node_list
            .clone()
            .then(
                choice((
                    just(',').to(BinaryGroupKind::Commutator),
                    just(':').to(BinaryGroupKind::Conjugate),
                ))
                .spanned(),
            )
            .then(node_list.clone())
            .delimited_by(just('['), just(']'))
            .map(|((a, kind), b)| RepeatableNode::BinaryGroup {
                kind,
                contents: [a, b],
            })
            .labelled("commutator notation group");

        let group = group_kind()
            .spanned()
            .then(node_list.delimited_by(just('('), just(')')).spanned())
            .labelled("parenthetical group");

        let rotation = just('@')
            .to_span()
            .then(family(0))
            .then(bracketed_transform().or_not())
            .map(|((at_sign, family), transform)| RepeatableNode::Rotation {
                at_sign,
                family,
                transform,
            })
            .contextual()
            .configure(|_, ctx: &Features| ctx.generalized_rotations)
            .labelled("rotation");

        let move_ = layer_prefix()
            .spanned()
            .then(family(1))
            .then(bracketed_transform().or_not())
            .map(|((layers, family), transform)| RepeatableNode::Move {
                layers,
                family,
                transform,
            })
            .labelled("move");

        let repeatable_node = choice((
            rotation,
            group.map(|(kind, nodes)| RepeatableNode::Group {
                kind,
                contents: nodes,
            }),
            commutator_or_conjugate,
            move_,
        ))
        .spanned()
        .then(multiplier().spanned())
        .map(|(inner, multiplier)| Node::RepeatedNode { inner, multiplier });

        let node = choice((
            just('.').to(Node::Pause),
            megaminx_scramble_move().map(Node::MegaminxScrambleMove),
            repeatable_node,
            sq1_move().map(Node::Sq1Move),
        ));

        node.spanned()
            .separated_by(text::whitespace().at_least(1))
            .collect()
            .padded()
            .map(NodeList)
    })
}

fn megaminx_scramble_move<'src>() -> impl NotationParser<'src, MegaminxScrambleMove> {
    choice((
        just("R++").to(MegaminxScrambleMove::Rpp),
        just("R--").to(MegaminxScrambleMove::Rmm),
        just("D++").to(MegaminxScrambleMove::Dpp),
        just("D--").to(MegaminxScrambleMove::Dmm),
    ))
    .labelled("Megaminx scramble move")
    .contextual()
    .configure(|_, ctx: &Features| ctx.megaminx)
}

fn sq1_move<'src>() -> impl NotationParser<'src, Sq1Move> {
    choice((
        just('/').to(Sq1Move::Slash),
        sint()
            .padded()
            .then_ignore(just(','))
            .then(sint().padded())
            .delimited_by(just('('), just(')'))
            .map(|(u, d)| Sq1Move::UD { u, d }),
    ))
    .labelled("Square-1 move")
    .contextual()
    .configure(|_, ctx: &Features| ctx.sq1)
}

fn bracketed_transform<'src>() -> impl NotationParser<'src, Spanned<Span>> {
    any()
        .filter(|&c| crate::charsets::is_bracketed_transform_char(c))
        .separated_by(just(' ').repeated())
        .to_span()
        .padded()
        .delimited_by(just('['), just(']'))
        .spanned()
}

fn family<'src>(min_len: usize) -> impl NotationParser<'src, Span> {
    any()
        .filter(|&c| crate::charsets::is_family_char(c))
        .repeated()
        .at_least(min_len)
        .to_span()
}

fn group_kind<'src>() -> impl NotationParser<'src, GroupKind> {
    one_of(crate::charsets::GROUP_PREFIX_CHARS)
        .try_map(|c, span| match c {
            '!' => Ok(GroupKind::Macro),
            '&' => Ok(GroupKind::Simultaneous),
            '^' => Ok(GroupKind::Niss),
            _ => Err(Rich::custom(span, format!("unknown group prefix: {c}"))),
        })
        .or_not()
        .map(Option::unwrap_or_default)
}

fn layer_prefix<'src>() -> impl NotationParser<'src, LayerPrefix> {
    let signed_layer_range = choice((
        signed_layer_range().map(|(i, j)| LayerSetElement::Range([i, j])),
        unsigned_layer_range()
            .contextual()
            .configure(|_, ctx: &Features| ctx.layers.hsc1_layer_ranges)
            .map(|(i, j)| LayerSetElement::Range([i, j])),
        sint().map(LayerSetElement::Single),
    ))
    .spanned()
    .separated_by(just(',').padded())
    .collect()
    .padded()
    .delimited_by(just('{'), just('}'))
    .contextual()
    .configure(|_, ctx: &Features| ctx.layers.layer_sets);

    let tilde = just('~')
        .to_span()
        .contextual()
        .configure(|_, ctx: &Features| ctx.layers.inverting);

    let layer_prefix_contents = choice((
        signed_layer_range.map(LayerPrefixContents::Set),
        unsigned_layer_range().map(|(i, j)| LayerPrefixContents::Range([i, j])),
        uint().map(LayerPrefixContents::Single),
    ));

    tilde
        .or_not()
        .then(layer_prefix_contents.spanned().or_not())
        .map(|(invert, contents)| LayerPrefix { invert, contents })
}

fn multiplier<'src>() -> impl NotationParser<'src, Multiplier> {
    uint().or_not().then(just('\'').ignored().or_not()).try_map(
        |(i, negate): (Option<i32>, Option<()>), span| match negate {
            Some(()) => match i.unwrap_or(1).checked_neg() {
                Some(i) => Ok(Multiplier(i)),
                None => Err(Rich::custom(span, "integer overflow")),
            },
            None => Ok(Multiplier(i.unwrap_or(1))),
        },
    )
}

/// Signed layer range, using `..` between the range endpoints.
fn signed_layer_range<'src, I: FromSignedInt>()
-> impl NotationParser<'src, (Spanned<I>, Spanned<I>)> {
    sint()
        .spanned()
        .then_ignore(just(".."))
        .then(sint().spanned())
}

/// Unsigned layer range, using `-` between the range endpoints.
fn unsigned_layer_range<'src, U: FromUnsignedInt>()
-> impl NotationParser<'src, (Spanned<U>, Spanned<U>)> {
    uint()
        .spanned()
        .then_ignore(just('-'))
        .then(uint().spanned())
}

/// Unsigned integer parser
fn uint<'src, U: FromUnsignedInt>() -> impl NotationParser<'src, U> {
    // allow leading zeros
    one_of('0'..='9')
        .repeated()
        .at_least(1)
        .to_slice()
        .try_map_with(|s: &str, e| s.parse().map_err(|err| Rich::custom(e.span(), err)))
}

/// Signed integer parser
fn sint<'src, I: FromSignedInt>() -> impl NotationParser<'src, I> {
    // allow leading zeros
    just('-')
        .or_not()
        .ignore_then(one_of('0'..='9').repeated().at_least(1))
        .to_slice()
        .try_map_with(|s: &str, e| s.parse().map_err(|err| Rich::custom(e.span(), err)))
}

trait FromUnsignedInt: FromStr<Err: ToString> {}
impl FromUnsignedInt for i32 {}
impl FromUnsignedInt for Layer {}
impl FromUnsignedInt for SignedLayer {}

trait FromSignedInt: FromStr<Err: ToString> {}
impl FromSignedInt for i32 {}
impl FromSignedInt for SignedLayer {}
