use std::{fmt, ops::Add, sync::Arc};

use eyre::{Context, eyre};
use hypermath::{
    AbsDiffEq, ApproxHashMapKey, Float, Hyperplane, TransformByMotor, Vector, pga::Motor,
};
use hyperpuzzle_core::{catalog::BuildTask, prelude::*};
use hyperpuzzlescript::*;
use hypershape::AbbrGenSeq;
use itertools::Itertools;

use super::{
    ArcMut, HpsAxis, HpsColor, HpsNdEuclid, HpsOrbitNames, HpsOrbitNamesComponent, HpsPuzzle,
    HpsShape, HpsSymmetry, HpsTwist, HpsTwistSystem,
};
use crate::builder::*;

impl_simple_custom_type!(HpsPuzzle = "euclid.Puzzle");
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

impl_simple_custom_type!(HpsTwistSystem = "euclid.TwistSystem");
impl fmt::Debug for HpsTwistSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
impl fmt::Display for HpsTwistSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(id = {:?})", self.type_name(), self.lock().id)
    }
}

impl hyperpuzzlescript::EngineCallback<PuzzleListMetadata, PuzzleSpec> for HpsNdEuclid {
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
            this.add_axes(ctx, vector, None, vec![], false)?
        }
        fn add_axis(
            ctx: EvalCtx,
            this: HpsPuzzle,
            vector: Vector,
            names: Names,
        ) -> Option<HpsAxis> {
            this.add_axes(ctx, vector, Some(names), vec![], false)?
        }
        #[kwargs(slice: Option<bool>)]
        fn add_axis(
            ctx: EvalCtx,
            this: HpsPuzzle,
            vector: Vector,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            this.add_axes(ctx, vector, None, layers, slice.unwrap_or(true))?
        }
        #[kwargs(slice: Option<bool>)]
        fn add_axis(
            ctx: EvalCtx,
            this: HpsPuzzle,
            vector: Vector,
            names: Names,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            this.add_axes(ctx, vector, Some(names), layers, slice.unwrap_or(true))?
        }
        #[kwargs(slice: Option<bool>)]
        fn add_axis(
            ctx: EvalCtx,
            this: HpsPuzzle,
            names: Names,
            vector: Vector,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            this.add_axes(ctx, vector, Some(names), layers, slice.unwrap_or(true))?
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
    fn add_axes(
        &self,
        ctx: &mut EvalCtx<'_>,
        vector: Vector,
        names: Option<Names>,
        layers: Vec<f64>,
        slice: bool,
    ) -> Result<Option<HpsAxis>> {
        let span = ctx.caller_span;
        let ctx_symmetry = ctx.scope.special.sym.ref_to::<Option<&HpsSymmetry>>()?;
        let mut this = self.lock();
        let mut twists = this.twists.lock();

        let (gen_seqs, transforms, vectors) = match ctx_symmetry {
            Some(sym) => sym.orbit(vector).into_iter().multiunzip(),
            None => (
                vec![AbbrGenSeq::INIT],
                vec![Motor::ident(this.ndim())],
                vec![vector],
            ),
        };

        let names = match names {
            Some(names) => names.0.to_strings(ctx, &transforms, span)?,
            None => vec![],
        }
        .into_iter()
        .map(Some)
        .chain(std::iter::repeat(None));

        // Add & name axes.
        let mut axes_list = vec![];
        for (transformed_vector, name) in std::iter::zip(&vectors, names) {
            let new_axis = twists.axes.add(transformed_vector.clone()).at(span)?;
            twists.axes.names.set(new_axis, name).at(span)?;
            axes_list.push(Some(new_axis));
        }
        let first_axis = axes_list.get(0).copied().flatten();
        drop(twists);

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
            for axis_vector in vectors {
                let mut shape = this.shape.lock();
                for &distance in &layers {
                    let layer_slice_plane = Hyperplane::new(&axis_vector, distance)
                        .ok_or("bad cut plane")
                        .at(span)?;
                    shape.slice(None, layer_slice_plane, None).at(span)?;
                }
            }
        }

        if ctx_symmetry.is_some() {
            this.twists.lock().axes.orbits.push(Orbit {
                elements: Arc::new(axes_list),
                generator_sequences: Arc::new(gen_seqs),
            });
        }

        Ok(first_axis.map(|id| HpsAxis {
            id,
            twists: ArcMut(Arc::clone(&this.twists)),
        }))
    }
}

impl HpsTwistSystem {
    /// Adds a new set of twists with symmetry and multipliers.
    fn add_symmetric_with_multipliers(
        &self,
        ctx: &mut EvalCtx<'_>,
        axis: HpsAxis,
        transform: Motor,
        kwargs: Map,
    ) -> Result<Option<HpsTwist>> {
        let span = ctx.caller_span;
        let ndim = ctx.ndim()?;

        unpack_kwargs!(
            kwargs,
            multipliers: Option<bool>,
            inverse: Option<bool>,
            prefix: Option<Names>,
            name: Option<Names>,
            suffix: Option<Names>,
            inv_name: Option<Names>,
            inv_suffix: Option<Names>,
            name_fn: Option<Arc<FnValue>>,
            qtm: Option<usize>,
            gizmo_pole_distance: Option<Num>,
        );

        let prefix = prefix.map(|Names(n)| n);
        let name = name.map(|Names(n)| n);
        let suffix = suffix.map(|Names(n)| n);
        let inv_name = inv_name.map(|Names(n)| n);
        let inv_suffix = inv_suffix.map(|Names(n)| n);

        let gizmo_pole_distance = gizmo_pole_distance.map(|x| x as f32);

        let axis_id = axis.id;
        let prefix = prefix.or_else(|| Some((HpsOrbitNamesComponent::Axis(axis), span).into()));
        let axis = axis_id;

        let do_naming = prefix.as_ref().is_some_and(|n| !n.is_empty())
            || name.as_ref().is_some_and(|n| !n.is_empty())
            || suffix.as_ref().is_some_and(|n| !n.is_empty())
            || inv_name.as_ref().is_some_and(|n| !n.is_empty())
            || inv_suffix.as_ref().is_some_and(|n| !n.is_empty())
            || name_fn.is_some();

        let inverse = inverse.unwrap_or(ndim == 3);
        let multipliers = multipliers.unwrap_or(ndim == 3);

        let suffix = suffix.unwrap_or_default();
        let inv_suffix = inv_suffix.unwrap_or_else(|| match &inv_name {
            Some(_) => suffix.clone(),
            None => HpsOrbitNames::from("'"),
        });

        if name_fn.is_some() && (name.is_some() || inv_name.is_some()) {
            return Err(
                "when `name_fn` is specified, `name` and `inv_name` must not be specified".at(span),
            );
        }

        let prefix = prefix.unwrap_or_default();
        let name = name.unwrap_or_default();
        let inv_name = inv_name.unwrap_or_else(|| name.clone());

        let qtm = qtm.unwrap_or(1);
        if qtm < 1 {
            ctx.warn("twist has QTM value less than 1");
        }

        if gizmo_pole_distance.is_some() && ndim != 3 && ndim != 4 {
            return Err("twist gizmo is only supported in 3D and 4D".at(span));
        }

        let base_transform = transform;

        let get_name = |ctx: &mut EvalCtx<'_>, i: i32| {
            if let Some(name_fn) = &name_fn {
                let args = vec![ValueData::Num(i as Num).at(span)];
                name_fn
                    .call(span, ctx, args, Map::new())?
                    .to()
                    .map(|Names(n)| n)
            } else if do_naming {
                match i {
                    1 => Ok(prefix.clone() + name.clone() + suffix.clone()),
                    -1 => Ok(prefix.clone() + inv_name.clone() + inv_suffix.clone()),
                    2.. => {
                        let mult = HpsOrbitNames::from(i.to_string().as_str());
                        Ok(prefix.clone() + name.clone() + mult.clone() + suffix.clone())
                    }
                    ..=-2 => {
                        let mult = HpsOrbitNames::from((-i).to_string().as_str());
                        Ok(prefix.clone() + inv_name.clone() + mult.clone() + inv_suffix.clone())
                    }
                    0 => Err("bad twist multiplier".at(span)),
                }
            } else {
                Ok(HpsOrbitNames::default())
            }
        };

        let transform = base_transform.clone();
        let builder = TwistBuilder {
            axis,
            transform,
            qtm,
            gizmo_pole_distance,
            include_in_scrambles: true,
        };
        let twist_name = get_name(ctx, 1)?;
        let first_twist_id = self.add_symmetric(ctx, builder, twist_name)?;
        if inverse {
            let transform = base_transform.reverse();
            let is_equivalent_to_reverse = base_transform.is_self_reverse();
            let twist_name = get_name(ctx, -1)?;
            let builder = TwistBuilder {
                axis,
                transform,
                qtm,
                gizmo_pole_distance: gizmo_pole_distance.filter(|_| ndim > 3),
                include_in_scrambles: !is_equivalent_to_reverse,
            };
            self.add_symmetric(ctx, builder, twist_name)?;
        }

        let mut previous_transform = base_transform.clone();
        for i in 2.. {
            if !multipliers {
                break;
            }

            // Check whether we've exceeded the max repeat count.
            if i > crate::MAX_TWIST_REPEAT as i32 {
                return Err(format!(
                    "twist transform takes too long to repeat! exceeded maximum of {}",
                    crate::MAX_TWIST_REPEAT,
                )
                .at(span));
            }

            let transform = &previous_transform * &base_transform;

            // Check whether we've reached the inverse.
            if inverse {
                if previous_transform.is_self_reverse()
                    || transform.is_equivalent_to(&previous_transform.reverse())
                {
                    break;
                }
            } else if transform.is_ident() {
                break;
            }
            previous_transform = transform.clone();

            let builder = TwistBuilder {
                axis,
                transform,
                qtm: qtm * i as usize,
                gizmo_pole_distance: None, // no gizmo for multiples
                include_in_scrambles: true,
            };
            let names = get_name(ctx, i)?;
            self.add_symmetric(ctx, builder, names)?;

            if inverse {
                let transform = previous_transform.reverse();
                let is_equivalent_to_reverse = previous_transform.is_self_reverse();
                let builder = TwistBuilder {
                    axis,
                    transform,
                    qtm: qtm * i as usize,
                    gizmo_pole_distance: None, // no gizmo for multiples
                    include_in_scrambles: !is_equivalent_to_reverse,
                };
                let names = get_name(ctx, -i)?;
                self.add_symmetric(ctx, builder, names)?;
            }
        }

        Ok(first_twist_id)
    }

    // Adds a set of symmetric twists.
    fn add_symmetric(
        &self,
        ctx: &mut EvalCtx<'_>,
        mut builder: TwistBuilder,
        names: HpsOrbitNames,
    ) -> Result<Option<HpsTwist>> {
        let span = ctx.caller_span;
        let ctx_symmetry = ctx.scope.special.sym.ref_to::<Option<&HpsSymmetry>>()?;

        let mut first_twist = None;

        match ctx_symmetry {
            Some(sym) => {
                let this = self.lock();
                let axis_vector = this.axes.get(builder.axis).at(span)?.vector().clone();
                let (transforms, orbit_elements): (Vec<_>, Vec<_>) = sym
                    .orbit(GeometricTwistKey {
                        axis_vector,
                        transform: builder.transform.clone(),
                    })
                    .into_iter()
                    .map(|(_gen_seq, transform, orbit_element)| (transform, orbit_element))
                    .unzip();

                drop(this); // unlock mutex before `to_strings()`
                let names = names.to_strings(ctx, &transforms, span)?;
                let mut this = self.lock();

                for (key, name) in std::iter::zip(orbit_elements, names) {
                    builder.axis =
                        super::axis_from_vector(&this.axes, &key.axis_vector).at(span)?;
                    builder.transform = key.transform;
                    let new_twist = this
                        .add_named(builder.clone(), Some(name), ctx.warnf())
                        .at(span)?;
                    if first_twist.is_none() {
                        first_twist = Some(new_twist);
                    }
                }
            }
            None => {
                let names = names.to_strings(ctx, &[Motor::ident(ctx.ndim()?)], span)?;
                let mut this = self.lock();
                first_twist = Some(
                    this.add_named(builder, names.into_iter().next(), ctx.warnf())
                        .at(span)?,
                );
            }
        };

        Ok(first_twist.flatten().map(|id| HpsTwist {
            id,
            twists: self.clone(),
        }))
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

#[derive(Debug, Clone)]
pub struct Names(HpsOrbitNames);
impl_ty!(Names = Type::Str | HpsOrbitNames::hps_ty());
impl FromValue for Names {
    fn from_value(value: Value) -> Result<Self> {
        let span = value.span;
        if value.as_ref::<str>().is_ok() {
            Ok(Self(HpsOrbitNames::from((value.to::<Str>()?.into(), span))))
        } else if value.as_ref::<HpsOrbitNames>().is_ok() {
            Ok(Self(value.to::<HpsOrbitNames>()?))
        } else if value.as_ref::<FnValue>().is_ok() {
            Ok(Self(HpsOrbitNames::from((
                HpsOrbitNamesComponent::Fn(value.to::<Arc<FnValue>>()?),
                span,
            ))))
        } else {
            Err(value.type_error(Self::hps_ty()))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct GeometricTwistKey {
    axis_vector: Vector,
    transform: Motor,
}
impl AbsDiffEq for GeometricTwistKey {
    type Epsilon = Float;

    fn default_epsilon() -> Self::Epsilon {
        hypermath::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.axis_vector.abs_diff_eq(&other.axis_vector, epsilon)
            && self.transform.abs_diff_eq(&other.transform, epsilon)
    }
}
impl TransformByMotor for GeometricTwistKey {
    fn transform_by(&self, m: &Motor) -> Self {
        let t = m.transform(&self.transform);
        Self {
            axis_vector: m.transform(&self.axis_vector),
            transform: if m.is_reflection() { t.reverse() } else { t },
        }
    }
}
impl ApproxHashMapKey for GeometricTwistKey {
    type Hash = (
        <Vector as ApproxHashMapKey>::Hash,
        <Motor as ApproxHashMapKey>::Hash,
    );

    fn approx_hash(
        &self,
        mut float_hash_fn: impl FnMut(Float) -> hypermath::collections::approx_hashmap::FloatHash,
    ) -> Self::Hash {
        (
            self.axis_vector.approx_hash(&mut float_hash_fn),
            self.transform.approx_hash(float_hash_fn),
        )
    }
}
