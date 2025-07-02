use std::{fmt, sync::Arc};

use hypermath::{Hyperplane, pga::Motor};
use hyperpuzzle_core::{Color, Orbit};
use hyperpuzzlescript::{
    Builtins, CustomValue, ErrorExt, EvalCtx, Result, hps_fns, impl_simple_custom_type,
};
use hypershape::AbbrGenSeq;
use itertools::Itertools;

use super::{ArcMut, HpsColor, HpsRegion, HpsSymmetry, Names};
use crate::builder::{ColorSystemBuilder, ShapeBuilder};

/// HPS shape builder.
pub(super) type HpsShape = ArcMut<ShapeBuilder>;
impl_simple_custom_type!(HpsShape = "euclid.Shape");
impl fmt::Debug for HpsShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
impl fmt::Display for HpsShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(ndim = {:?})", self.type_name(), self.lock().ndim())
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsShape>()?;

    builtins.set_fns(hps_fns![
        #[kwargs(region: Option<HpsRegion>, color: Option<HpsColor>)]
        fn carve(ctx: EvalCtx, plane: Hyperplane) -> Option<HpsColor> {
            let sticker_mode = match color {
                None => StickerMode::NewColor,
                Some(c) => StickerMode::FixedColor(c),
            };
            let args = CutArgs::carve(sticker_mode, region);
            HpsShape::get(ctx)?.cut(ctx, plane, args)?
        }
        #[kwargs(region: Option<HpsRegion>)]
        fn carve(ctx: EvalCtx, plane: Hyperplane, color_names: Names) -> Option<HpsColor> {
            let args = CutArgs::carve(StickerMode::FromNames(color_names), region);
            HpsShape::get(ctx)?.cut(ctx, plane, args)?
        }

        #[kwargs(region: Option<HpsRegion>)]
        fn slice(ctx: EvalCtx, plane: Hyperplane) -> Option<HpsColor> {
            let args = CutArgs::slice(StickerMode::None, region);
            HpsShape::get(ctx)?.cut(ctx, plane, args)?
        }
        #[kwargs(region: Option<HpsRegion>)]
        fn slice(ctx: EvalCtx, plane: Hyperplane, color_names: Names) -> Option<HpsColor> {
            let args = CutArgs::slice(StickerMode::FromNames(color_names), region);
            HpsShape::get(ctx)?.cut(ctx, plane, args)?
        }

        fn add_piece_type(ctx: EvalCtx, name: String) -> () {
            ignore_ctx_symmetry(ctx);
            let shape = HpsShape::get(ctx)?;
            if let Err(e) = shape.lock().get_or_add_piece_type(name, None) {
                ctx.warn(e.to_string());
            }
        }
        fn add_piece_type(ctx: EvalCtx, name: String, display: String) -> () {
            ignore_ctx_symmetry(ctx);
            let shape = HpsShape::get(ctx)?;
            if let Err(e) = shape.lock().get_or_add_piece_type(name, Some(display)) {
                ctx.warn(e.to_string());
            }
        }

        fn mark_piece(ctx: EvalCtx, region: HpsRegion, name: String) -> () {
            ignore_ctx_symmetry(ctx);
            let shape = HpsShape::get(ctx)?;
            let result = shape.lock().mark_piece_by_region(
                &name,
                None,
                |point| region.contains_point(point),
                ctx.warnf(),
            );
            result.at(ctx.caller_span)?;
        }
        fn mark_piece(ctx: EvalCtx, region: HpsRegion, name: String, display: String) -> () {
            ignore_ctx_symmetry(ctx);
            let shape = HpsShape::get(ctx)?;
            let result = shape.lock().mark_piece_by_region(
                &name,
                Some(display),
                |point| region.contains_point(point),
                ctx.warnf(),
            );
            result.at(ctx.caller_span)?;
        }

        fn unify_piece_types(ctx: EvalCtx, sym: HpsSymmetry) -> () {
            ignore_ctx_symmetry(ctx);
            let shape = HpsShape::get(ctx)?;
            shape
                .lock()
                .unify_piece_types(&sym.generators(), &mut ctx.warnf())
        }

        fn delete_untyped_pieces(ctx: EvalCtx) -> () {
            let shape = HpsShape::get(ctx)?;
            shape.lock().delete_untyped_pieces(&mut ctx.warnf())
        }

        fn autoname_colors(ctx: EvalCtx, shape: HpsShape) -> () {
            let mut shape = shape.lock();
            let len = shape.colors.len();
            shape
                .colors
                .names
                .autoname(len, ColorSystemBuilder::autonames())
                .at(ctx.caller_span)?;
        }
    ])
}

impl HpsShape {
    pub fn get<'a>(ctx: &EvalCtx<'a>) -> Result<&'a Self> {
        ctx.scope.special.shape.as_ref()
    }

    fn cut(
        &self,
        ctx: &mut EvalCtx<'_>,
        plane: Hyperplane,
        args: CutArgs,
    ) -> Result<Option<HpsColor>> {
        let span = ctx.caller_span;
        let ctx_symmetry = HpsSymmetry::get(ctx)?;
        let mut this = self.lock();

        let (gen_seqs, transforms, cut_planes): (Vec<_>, Vec<_>, Vec<_>) = match ctx_symmetry {
            Some(sym) => sym.orbit(plane).into_iter().multiunzip(),
            None => (
                vec![AbbrGenSeq::INIT],
                vec![Motor::ident(this.ndim())],
                vec![plane],
            ),
        };

        let mut fixed_color: Option<Color> = None;
        let mut color_list: Option<Vec<Option<Color>>> = None;
        match args.stickers {
            StickerMode::NewColor => {
                color_list = Some(
                    (0..cut_planes.len())
                        .map(|_| this.colors.add().map(Some))
                        .try_collect()
                        .at(span)?,
                );
            }
            StickerMode::None => fixed_color = None,
            StickerMode::FixedColor(c) => fixed_color = Some(c.id),
            StickerMode::FromNames(names) => {
                let color_names = names.0.to_strings(ctx, &transforms, span)?;
                color_list = Some(
                    color_names
                        .into_iter()
                        .map(|name_spec| {
                            name_spec
                                .map(|s| this.colors.get_or_add_with_name_spec(s, &mut ctx.warnf()))
                                .transpose()
                        })
                        .try_collect()
                        .at(span)?,
                );
            }
        };

        Ok(match color_list {
            Some(colors) => {
                if ctx_symmetry.is_some() {
                    this.colors.orbits.push(Orbit {
                        elements: Arc::new(colors.clone()),
                        generator_sequences: Arc::new(gen_seqs),
                    });
                }
                drop(this);
                self.cut_all(args.mode, args.region, std::iter::zip(cut_planes, colors))
            }
            None => {
                let colors = std::iter::repeat(fixed_color);
                drop(this);
                self.cut_all(args.mode, args.region, std::iter::zip(cut_planes, colors))
            }
        }
        .at(span)?
        .map(|id| {
            let shape = self.clone();
            HpsColor { id, shape }
        }))
    }

    fn cut_all(
        &self,
        mode: CutMode,
        region: Option<HpsRegion>,
        orbit: impl IntoIterator<Item = (Hyperplane, Option<Color>)>,
    ) -> eyre::Result<Option<Color>> {
        let mut first_color = None;

        let mut this = self.lock();

        let piece_set = region.as_ref().map(|r| {
            this.active_pieces_in_region(|point| r.contains_point(point))
                .collect()
        });

        for (cut_plane, color) in orbit {
            first_color.get_or_insert(color);
            match mode {
                CutMode::Carve => this.carve(piece_set.as_ref(), cut_plane, color)?,
                CutMode::Slice => this.slice(piece_set.as_ref(), cut_plane, color)?,
            }
        }

        Ok(first_color.flatten())
    }
}

/// Cut arguments.
#[derive(Debug)]
struct CutArgs {
    mode: CutMode,
    stickers: StickerMode,
    region: Option<HpsRegion>,
}
impl CutArgs {
    fn carve(stickers: StickerMode, region: Option<HpsRegion>) -> Self {
        Self {
            mode: CutMode::Carve,
            stickers,
            region,
        }
    }
    fn slice(stickers: StickerMode, region: Option<HpsRegion>) -> Self {
        Self {
            mode: CutMode::Slice,
            stickers,
            region,
        }
    }
}

/// Which pieces to keep when cutting the shape.
#[derive(Debug)]
enum CutMode {
    /// Delete any pieces outside the cut; keep only pieces inside the cut.
    Carve,
    /// Keep all pieces on both sides of the cut.
    Slice,
}

/// How to sticker new facets created by a cut.
#[derive(Debug, Default)]
enum StickerMode {
    /// Add a new color for each cut and create new stickers with that color on
    /// both sides of the cut.
    #[default]
    NewColor,
    /// Do not add new stickers.
    None,
    /// Add new stickers using an existing color.
    FixedColor(HpsColor),
    /// Add new stickers using orbit names.
    FromNames(Names),
}

fn ignore_ctx_symmetry(ctx: &mut EvalCtx<'_>) {
    if !ctx.scope.special.sym.is_null() {
        ctx.warn("ignoring global symmetry");
    }
}
