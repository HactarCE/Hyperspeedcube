use std::{fmt, sync::Arc};

use eyre::{Context, eyre};
use hypermath::{Float, Hyperplane, Vector, pga::Motor};
use hyperpuzzle_core::{catalog::BuildTask, prelude::*};
use hyperpuzzlescript::*;
use hypershape::AbbrGenSeq;
use itertools::Itertools;

use super::{
    ArcMut, HpsAxis, HpsColor, HpsNdEuclid, HpsPuzzle, HpsShape, HpsSymmetry, HpsTwist, Names,
};
use crate::builder::*;

impl_simple_custom_type!(HpsPuzzle = "euclid.Puzzle", field_get = Self::field_get);
impl fmt::Debug for HpsPuzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
impl fmt::Display for HpsPuzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(id = {:?})", self.type_name(), self.lock().meta.id)
    }
}

impl hyperpuzzlescript::EngineCallback<PuzzleListMetadata, PuzzleSpec> for HpsNdEuclid {
    fn name(&self) -> String {
        self.to_string()
    }

    fn new(
        &self,
        ctx: &mut EvalCtx<'_>,
        mut meta: PuzzleListMetadata,
        kwargs: Map,
        catalog: Catalog,
        eval_tx: EvalRequestTx,
    ) -> Result<PuzzleSpec> {
        let caller_span = ctx.caller_span;

        unpack_kwargs!(
            kwargs,
            colors: Option<String>, // TODO: string or array of strings (gen ID + params)
            twists: Option<String>, // TODO: string or array of strings (gen ID + params)
            ndim: u8,
            (build, build_span): Arc<FnValue>,
            remove_internals: Option<bool>,
            scramble: Option<u32>,
        );

        if let Some(color_system_id) = colors.clone() {
            meta.tags
                .insert_named("colors/system", TagValue::Str(color_system_id.into()))
                .map_err(|e| Error::User(e.to_string().into()).at(caller_span))?;
        }

        if let Some(twist_system_id) = twists.clone() {
            meta.tags
                .insert_named("twists/system", TagValue::Str(twist_system_id.into()))
                .map_err(|e| Error::User(e.to_string().into()).at(caller_span))?;
        }

        let meta = Arc::new(meta);

        Ok(PuzzleSpec {
            meta: Arc::clone(&meta),
            build: Box::new(move |build_ctx| {
                let builder = ArcMut::new(PuzzleBuilder::new(Arc::clone(&meta), ndim)?);

                // Build color system.
                if let Some(color_system_id) = &colors {
                    build_ctx.progress.lock().task = BuildTask::BuildingColors;
                    let colors = catalog
                        .build_blocking(color_system_id)
                        .map_err(|e| eyre!(e))?;
                    builder.shape().lock().colors = ColorSystemBuilder::unbuild(&colors)?;
                }

                // Build twist system.
                if let Some(twist_system_id) = &twists {
                    build_ctx.progress.lock().task = BuildTask::BuildingTwists;
                    let twists = catalog
                        .build_blocking(twist_system_id)
                        .map_err(|e| eyre!(e))?;
                    *builder.twists().lock() = TwistSystemBuilder::unbuild(&twists)?;
                }

                build_ctx.progress.lock().task = BuildTask::BuildingPuzzle;

                if let Some(remove_internals) = remove_internals {
                    builder.shape().lock().remove_internals = remove_internals;
                }
                if let Some(full_scramble_length) = scramble {
                    builder.lock().full_scramble_length = full_scramble_length
                        .try_into()
                        .wrap_err("bad scramble length")?;
                };

                let mut scope = Scope::default();
                scope.special.ndim = Some(ndim);
                let scope = Arc::new(scope);

                let build_fn = Arc::clone(&build);

                eval_tx.eval_blocking(move |runtime| {
                    let mut ctx = EvalCtx {
                        scope: &scope,
                        runtime,
                        caller_span,
                        exports: &mut None,
                    };
                    build_fn
                        .call(
                            build_span,
                            &mut ctx,
                            vec![builder.clone().at(caller_span)],
                            Map::new(),
                        )
                        .map_err(|e| {
                            let s = e.to_string(&*ctx.runtime);
                            ctx.runtime.report_diagnostic(e);
                            eyre!(s)
                        })?;

                    let b = builder.lock();

                    // Assign default piece type to remaining pieces.
                    b.shape.lock().mark_untyped_pieces()?;

                    b.build(Some(&build_ctx), &mut ctx.warnf())
                        .map(|ok| Redirectable::Direct(ok))
                })
            }),
        })
    }
}

/// Adds the built-ins to the scope.
pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_custom_type::<HpsPuzzle>();

    scope.register_builtin_functions(hps_fns![
        fn carve(ctx: EvalCtx, this: HpsPuzzle, plane: Hyperplane) -> Option<HpsColor> {
            let args = CutArgs::carve(StickerMode::NewColor);
            this.shape().cut(ctx, plane, args)?
        }
        fn carve(
            ctx: EvalCtx,
            this: HpsPuzzle,
            plane: Hyperplane,
            color_names: Names,
        ) -> Option<HpsColor> {
            let args = CutArgs::carve(StickerMode::FromNames(color_names));
            this.shape().cut(ctx, plane, args)?
        }
        fn carve(
            ctx: EvalCtx,
            this: HpsPuzzle,
            plane: Hyperplane,
            color: Option<HpsColor>,
        ) -> Option<HpsColor> {
            let args = CutArgs::carve(color.map_or(StickerMode::None, StickerMode::FixedColor));
            this.shape().cut(ctx, plane, args)?
        }

        fn slice(ctx: EvalCtx, this: HpsPuzzle, plane: Hyperplane) -> Option<HpsColor> {
            let args = CutArgs::slice(StickerMode::None);
            this.shape().cut(ctx, plane, args)?
        }
        fn slice(
            ctx: EvalCtx,
            this: HpsPuzzle,
            plane: Hyperplane,
            color_names: Names,
        ) -> Option<HpsColor> {
            let args = CutArgs::slice(StickerMode::FromNames(color_names));
            this.shape().cut(ctx, plane, args)?
        }

        fn add_axis(ctx: EvalCtx, this: HpsPuzzle, vector: Vector) -> Option<HpsAxis> {
            this.add_layered_axes(ctx, vector, None, vec![], false)?
        }
        fn add_axis(
            ctx: EvalCtx,
            this: HpsPuzzle,
            vector: Vector,
            names: Names,
        ) -> Option<HpsAxis> {
            this.add_layered_axes(ctx, vector, Some(names), vec![], false)?
        }
        #[kwargs(slice: bool = true)]
        fn add_axis(
            ctx: EvalCtx,
            this: HpsPuzzle,
            vector: Vector,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            this.add_layered_axes(ctx, vector, None, layers, slice)?
        }
        #[kwargs(slice: bool = true)]
        fn add_axis(
            ctx: EvalCtx,
            this: HpsPuzzle,
            vector: Vector,
            names: Names,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            this.add_layered_axes(ctx, vector, Some(names), layers, slice)?
        }
        #[kwargs(slice: bool = true)]
        fn add_axis(
            ctx: EvalCtx,
            this: HpsPuzzle,
            names: Names,
            vector: Vector,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            this.add_layered_axes(ctx, vector, Some(names), layers, slice)?
        }

        #[kwargs(kwargs)]
        fn add_twist(
            ctx: EvalCtx,
            this: HpsPuzzle,
            axis: HpsAxis,
            transform: Motor,
        ) -> Option<HpsTwist> {
            this.twists()
                .add_symmetric_with_multipliers(ctx, axis, transform, kwargs)?
        }
        #[kwargs(kwargs)]
        fn add_twist(
            ctx: EvalCtx,
            this: HpsPuzzle,
            axis: HpsAxis,
            transform: Motor,
            (name, name_span): Names,
        ) -> Option<HpsTwist> {
            if let Some(old_value) = kwargs.insert("name".into(), name.0.at(name_span)) {
                return Err("duplicate `name` argument".at(old_value.span));
            };
            this.twists()
                .add_symmetric_with_multipliers(ctx, axis, transform, kwargs)?
        }

        #[kwargs(slice: bool = true)]
        fn add_layers(
            ctx: EvalCtx,
            this: HpsPuzzle,
            (axis, axis_span): HpsAxis,
            layers: Vec<Num>,
        ) -> () {
            this.add_layers(ctx, (axis, axis_span), layers, slice)?;
        }
    ])
}

impl HpsShape {
    fn cut(
        &self,
        ctx: &mut EvalCtx<'_>,
        plane: Hyperplane,
        args: CutArgs,
    ) -> Result<Option<HpsColor>> {
        let span = ctx.caller_span;
        let ctx_symmetry = ctx.scope.special.sym.ref_to::<Option<&HpsSymmetry>>()?;
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
                            this.colors
                                .get_or_add_with_name_spec(name_spec, &mut ctx.warnf())
                                .map(Some)
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
                self.cut_all(args.mode, std::iter::zip(cut_planes, colors))
            }
            None => {
                let colors = std::iter::repeat(fixed_color);
                drop(this);
                self.cut_all(args.mode, std::iter::zip(cut_planes, colors))
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
        orbit: impl IntoIterator<Item = (Hyperplane, Option<Color>)>,
    ) -> eyre::Result<Option<Color>> {
        let mut first_color = None;

        let mut this = self.lock();
        for (cut_plane, color) in orbit {
            first_color.get_or_insert(color);
            match mode {
                CutMode::Carve => this.carve(None, cut_plane, color)?,
                CutMode::Slice => this.slice(None, cut_plane, color)?,
            }
        }

        Ok(first_color.flatten())
    }
}

impl HpsPuzzle {
    fn field_get(
        &self,
        _span: Span,
        (field, field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(match field {
            "twists" => Some(self.twists().into()),
            "axes" => Some(ValueData::Map(Arc::new(
                self.twists().axis_name_map(field_span),
            ))),
            _ => None,
        })
    }

    fn add_layered_axes(
        &self,
        ctx: &mut EvalCtx<'_>,
        vector: Vector,
        names: Option<Names>,
        layers: Vec<f64>,
        slice: bool,
    ) -> Result<Option<HpsAxis>> {
        // TODO: simplify this function. just call `add_axes()` and `add_layers()`

        // Add axes.
        let axes_list = self.twists().add_axes(ctx, vector, names)?;

        let span = ctx.caller_span;
        let mut this = self.lock();

        // Add layers.
        let axis_layers = this.axis_layers().at(span)?;
        for &axis in axes_list.iter().filter_map(Option::as_ref) {
            let axis_layers = &mut axis_layers[axis].0;
            for (&top, &bottom) in layers.iter().tuple_windows() {
                axis_layers
                    .push(AxisLayerBuilder { top, bottom })
                    .at(span)?;
            }
        }

        // Slice layers.
        if slice {
            let mut shape = this.shape.lock();
            let twists = this.twists.lock();
            for &axis in axes_list.iter().filter_map(Option::as_ref) {
                if let Ok(axis) = twists.axes.get(axis) {
                    let axis_vector = axis.vector();
                    for &distance in &layers {
                        let layer_slice_plane = Hyperplane::new(&axis_vector, distance)
                            .ok_or("bad cut plane")
                            .at(span)?;
                        shape.slice(None, layer_slice_plane, None).at(span)?;
                    }
                }
            }
        }

        let first_axis = axes_list.first().copied().flatten();

        Ok(first_axis.map(|id| HpsAxis {
            id,
            twists: ArcMut(Arc::clone(&this.twists)),
        }))
    }

    fn add_layers(
        &self,
        ctx: &mut EvalCtx<'_>,
        (axis, axis_span): Spanned<HpsAxis>,
        layers: Vec<Float>,
        slice: bool,
    ) -> Result<()> {
        let span = ctx.caller_span;
        let ctx_symmetry = ctx.scope.special.sym.ref_to::<Option<&HpsSymmetry>>()?;

        let axis_vector = axis.vector().at(axis_span)?;

        let axis_vectors: Vec<Vector> = match ctx_symmetry {
            Some(sym) => sym
                .orbit(axis_vector)
                .into_iter()
                .map(|(_gen_seq, _transform, v)| v)
                .collect(),
            None => vec![axis_vector],
        };

        let mut this = self.lock();
        let twists = this.twists.lock();
        let axes = &twists.axes;
        let axes: Vec<Axis> = axis_vectors
            .iter()
            .map(|v| super::axis_from_vector(axes, v))
            .try_collect()
            .at(span)?;
        drop(twists);

        // Add layers.
        let axis_layers = this.axis_layers().at(span)?;
        for axis in axes {
            let axis_layers = &mut axis_layers[axis].0;
            for (&top, &bottom) in layers.iter().tuple_windows() {
                axis_layers
                    .push(AxisLayerBuilder { top, bottom })
                    .at(span)?;
            }
        }

        // Slice layers.
        if slice {
            let mut shape = this.shape.lock();
            for v in axis_vectors {
                for &distance in &layers {
                    let layer_slice_plane = Hyperplane::new(&v, distance)
                        .ok_or("bad cut plane")
                        .at(span)?;
                    shape.slice(None, layer_slice_plane, None).at(span)?;
                }
            }
        }

        Ok(())
    }
}

/// Cut arguments.
#[derive(Debug)]
struct CutArgs {
    mode: CutMode,
    stickers: StickerMode,
    region: Option<std::convert::Infallible>, // TODO
}
impl CutArgs {
    pub fn carve(stickers: StickerMode) -> Self {
        Self {
            mode: CutMode::Carve,
            stickers,
            region: None,
        }
    }
    pub fn slice(stickers: StickerMode) -> Self {
        Self {
            mode: CutMode::Slice,
            stickers,
            region: None,
        }
    }
    pub fn with_region(mut self, region: Option<std::convert::Infallible>) -> Self {
        self.region = region;
        self
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
