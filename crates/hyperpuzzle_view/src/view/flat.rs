use std::sync::Arc;

use hyperdraw::GraphicsState;
use hyperprefs::{AnimationPreferences, Preferences};
use hyperpuzzle::prelude::*;
use parking_lot::Mutex;
use smallvec::smallvec;

use super::PuzzleViewInput;
use crate::styles::PuzzleStyleStates;
use crate::{PuzzleSimulation, ReplayEvent};

#[derive(Debug)]
pub struct FlatViewState {
    pub puzzle: Arc<Puzzle>,
    /// Flat puzzle geometry.
    pub geom: Arc<FlatPuzzleGeometry>,

    pub sticker_coords: PerSticker<[f32; hyperpuzzle::flat::MAX_NDIM]>,
    pub max_size: f32,

    pub key_queue: Vec<char>,
}

impl FlatViewState {
    /// Constructs a fresh state.
    ///
    /// Returns `None` if `puzzle` is not a Flat puzzle.
    pub fn new(
        gfx: &Arc<GraphicsState>,
        prefs: &mut Preferences,
        puzzle: &Arc<Puzzle>,
    ) -> Option<Self> {
        let puzzle = Arc::clone(&puzzle);
        let geom = Arc::clone(&puzzle.ui_data.downcast_ref::<FlatPuzzleUiData>()?.geom);
        let piece_positions: PerPiece<[u8; _]> =
            hyperpuzzle::flat::iter_piece_positions(geom.puzzle.dimensions).collect();
        let max_size = geom.puzzle.dimensions.into_iter().max().unwrap_or(1) as f32;
        let sticker_coords = puzzle.stickers.map_ref(|sticker, sticker_info| {
            let mut pos = piece_positions[sticker_info.piece].map(|coord| coord as f32);
            let facet = hyperpuzzle::flat::Facet::from_color(sticker_info.color);
            pos[facet.dim().0 as usize] += facet.sign().to_num::<f32>();
            pos.map(|coord| coord + 0.5 - max_size * 0.5)
        });
        // let renderer = NdEuclidPuzzleRenderer::new(gfx, puzzle)?;

        // let view_preset = prefs
        //     .perspective_view_presets(PerspectiveDim::from_ndim(geom.ndim()))
        //     .load_last_loaded(hyperprefs::DEFAULT_PRESET_NAME);

        Some(Self {
            puzzle,
            geom,
            max_size,
            sticker_coords,
            key_queue: vec![],
        })
    }

    /// Updates the puzzle view for a frame. This method is idempotent.
    pub fn update(
        &mut self,
        input: PuzzleViewInput,
        prefs: &Preferences,
        animation_prefs: &AnimationPreferences,
        sim: &Mutex<PuzzleSimulation>,
        styles: &PuzzleStyleStates,
    ) {
        let PuzzleViewInput {
            ndc_cursor_pos: _,
            target_size: _,
            is_dragging: _,
            exceeded_twist_drag_threshold: _,
            hover_mode: _,
            keys,
        } = input;

        for key in keys {
            if !"FSEDLJIKX".contains(key) {
                continue;
            }
            if self.key_queue.len() < 2 {
                self.key_queue.push(key);
                continue;
            }
            let facet = self.key_queue[0];
            let from = self.key_queue[1];
            let to = key;
            self.key_queue.clear();

            fn key_to_facet(key: char) -> Option<hyperpuzzle::flat::Facet> {
                "FSEDLJIK"
                    .find(key)
                    .map(|i| hyperpuzzle::flat::Facet(i as u8))
            }

            let Some(facet) = key_to_facet(facet) else {
                continue;
            };
            let Some(mut from) = key_to_facet(from) else {
                continue;
            };
            let Some(mut to) = key_to_facet(to) else {
                continue;
            };

            if from.sign() != to.sign() {
                std::mem::swap(&mut from, &mut to);
            }

            let Some(transform) = self
                .puzzle
                .twists
                .engine_data
                .downcast_ref::<hyperpuzzle::flat::FlatTwistSystemEngineData>()
                .unwrap()
                .twist_geometry_infos
                .find(|_, data| *data == (facet, from.dim(), to.dim()))
            else {
                continue;
            };

            sim.lock()
                .do_event(ReplayEvent::Twists(smallvec![LayeredTwist {
                    layers: LayerMask(1),
                    transform
                }]));
        }

        // for (i, pos) in
        //     hyperpuzzle::flat::iter_piece_positions(self.geom.puzzle.dimensions).enumerate()
        // {
        //     pos
        // }

        // // Convert NDC to screen space.
        // let cursor_pos = (|| {
        //     // IIFE to mimic try_block
        //     let [ndc_x, ndc_y] = ndc_cursor_pos?;
        //     let s = self.camera.xy_scale().ok()?;
        //     Some(cgmath::point2(ndc_x / s.x, ndc_y / s.y))
        // })();
        // // Update cursor position.
        // let cursor_delta = Option::zip(cursor_pos, self.cursor_pos).map(|(old, new)| new - old);
        // self.cursor_pos = cursor_pos;

        // // Update drag state.
        // if let Some(drag_state) = &mut self.drag_state {
        //     let mut sim = sim.lock();
        //     let puzzle = sim.puzzle();
        //     let nd_euclid = sim.nd_euclid();
        //     let ndim = match nd_euclid {
        //         Some(nd_euclid) => nd_euclid.geom.ndim(),
        //         None => 2,
        //     };

        //     match drag_state {
        //         // Update camera.
        //         DragState::ViewRot { z_axis } => {
        //             if let Some(mut delta) = cursor_delta {
        //                 if *z_axis > 2 {
        //                     delta = -delta;
        //                 }
        //                 if *z_axis < ndim {
        //                     let cgmath::Vector2 { x: dx, y: dy } = delta;
        //                     self.camera.rot =
        //                         pga::Motor::from_angle_in_axis_plane(0, *z_axis, dx as _)
        //                             * pga::Motor::from_angle_in_axis_plane(1, *z_axis, dy as _)
        //                             * &self.camera.rot;
        //                 }
        //             }
        //         }

        //         // Initialize partial twist state.
        //         DragState::PreTwist => {
        //             if exceeded_twist_drag_threshold && ndim == 3 {
        //                 // IIFE to mimic try_block
        //                 let best_grip = (|| {
        //                     let hov = self.puzzle_hover_state()?;
        //                     let parallel_drag_delta = self.parallel_drag_delta()?;
        //                     let target = hov.normal_3d().cross_product_3d(&parallel_drag_delta);
        //                     sim.nd_euclid()?
        //                         .geom
        //                         .axis_vectors
        //                         .iter()
        //                         .filter_map(|(axis, axis_vector)| {
        //                             // TODO: canoncalize axis based on layer mask
        //                             let layers = puzzle.min_drag_layer_mask(axis, hov.piece)?;
        //                             let score = target.dot(axis_vector.normalize()?).abs();
        //                             if !APPROX.is_pos(score) {
        //                                 return None;
        //                             }
        //                             Some((axis, layers, score))
        //                         })
        //                         // TODO: handle multiple good matches, maybe?
        //                         .max_by_key(|(_, _, score)| FloatOrd(*score))
        //                         .map(|(axis, layers, _)| (axis, layers))
        //                 })();
        //                 if let Some((axis, layers)) = best_grip {
        //                     sim.begin_nd_euclid_partial_twist(ndim, axis, layers);
        //                     self.drag_state = Some(DragState::Twist);
        //                 } else {
        //                     log::trace!("canceling partial twist");
        //                     self.drag_state = Some(DragState::Canceled);
        //                 }
        //             }
        //         }

        //         // Update partial twist state.
        //         DragState::Twist => {
        //             // IIFE to mimic try_block
        //             let partial_twist = (|| {
        //                 let hov = self.puzzle_hover_state()?;
        //                 let mut parallel_drag_delta = self.parallel_drag_delta()?;
        //                 let axis = nd_euclid?.partial_twist_drag_state.as_ref()?.axis;
        //                 let axis_vector = &nd_euclid?.geom.axis_vectors[axis];
        //                 let drag_origin = Point::ORIGIN; // TODO: change for multi-origin puzzles
        //                 if prefs.interaction.scale_twist_drag_by_radius {
        //                     parallel_drag_delta = parallel_drag_delta
        //                         / (hov.position.rejected_from(axis_vector)? - drag_origin).mag();
        //                 }
        //                 Some((hov.normal_3d(), parallel_drag_delta))
        //             })();
        //             if let Some((surface_normal, parallel_drag_delta)) = partial_twist {
        //                 sim.update_nd_euclid_partial_twist(
        //                     surface_normal,
        //                     parallel_drag_delta,
        //                     animation_prefs,
        //                 );
        //             }
        //         }

        //         DragState::Canceled => (),
        //     }
        // } else {
        //     // Update hover states, only when not in the middle of a drag.
        //     // IIFE to mimic try_block
        //     self.puzzle_hover_state = (|| {
        //         let vertex_3d_positions = self.renderer.puzzle_vertex_3d_positions.get()?;
        //         self.compute_sticker_hover_state(&vertex_3d_positions, prefs, styles, sim)
        //     })();
        //     self.gizmo_hover_state = (|| {
        //         let vertex_3d_positions = self.renderer.gizmo_vertex_3d_positions.get()?;
        //         self.compute_gizmo_hover_state(&vertex_3d_positions)
        //     })();
        // }

        // // Update camera.
        // self.camera.target_size = target_size;
    }
}
