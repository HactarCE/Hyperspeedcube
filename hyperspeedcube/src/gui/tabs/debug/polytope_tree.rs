use std::collections::HashSet;

use anyhow::{Context, Result};
use ndpuzzle::geometry::*;
use ndpuzzle::math::{cga, VectorRef};
use ndpuzzle::vector;

use crate::gui::tabs::puzzle_view::Overlay;
use crate::gui::App;

#[derive(Debug, Default)]
pub struct PolytopeTree {
    custom_number: u32,
    vis: HashSet<ShapeId>,
    hovered: Option<ShapeId>,
}

impl PolytopeTree {
    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let Some(puzzle_view) = app.active_puzzle_view.upgrade() else {
            return;
        };
        let mut mutex_guard = puzzle_view.lock();
        let puzzle_view = &mut *mutex_guard;
        let shapes = &puzzle_view.arena;
        let space_ndim = shapes.space().ndim().unwrap_or(0);

        self.hovered = None;

        let r = ui.add(egui::DragValue::new(&mut self.custom_number));
        if r.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))
            || ui.button("+").clicked()
        {
            let id = ShapeId(self.custom_number);
            self.set_state_recursively(shapes, id, !self.vis.contains(&id));
        }

        for &root in shapes.roots() {
            self.shape_ui(ui, shapes, root.into());
        }

        let overlay = &mut puzzle_view.overlay;
        overlay.clear();
        let mut append_overlay = |id: ShapeId, size: f32| -> Result<()> {
            let m = &shapes[id].manifold;
            match m.ndim()? {
                0 => {
                    let [a, b] = m.to_point_pair()?;
                    for p in [a.clone(), b.clone()] {
                        if let Ok(p) = p.to_finite() {
                            let color = egui::Color32::BLUE;
                            overlay.push((Overlay::Point(p), size, color));
                        }
                    }
                    let vector = (m.ipns() ^ cga::Blade::NO)
                        .ipns_to_opns(shapes.space().ndim()?)
                        .to_vector()
                        .normalize();
                    let color = egui::Color32::DARK_RED;
                    if let Some(v) = vector {
                        let v = v / 2.0;
                        if let Ok(a) = a.clone().to_finite() {
                            overlay.push((Overlay::Arrow(a.clone(), &a + &v), 1.0, color));
                        }
                        if let Ok(b) = b.clone().to_finite() {
                            overlay.push((Overlay::Arrow(&b - &v, b.clone()), 1.0, color));
                        }
                        if a.to_finite().is_err() && b.to_finite().is_err() {
                            overlay.push((Overlay::Arrow(vector![], v), 1.0, color));
                        }
                    }
                }
                1 if m.is_flat() => {
                    let [p1, p2] = if let Some(b) = shapes[id].boundary.iter().next() {
                        shapes.signed_manifold_of_shape(b)?.to_point_pair()?
                    } else {
                        let p = ndpuzzle::math::cga::Point::Infinity;
                        [p.clone(), p]
                    };
                    let [backup_p1, backup_p2] = (shapes[id].manifold.ipns()
                        ^ ndpuzzle::math::cga::Blade::ipns_sphere(vector![], 5.0))
                    .ipns_to_opns(space_ndim)
                    .point_pair_to_points()
                    .context("bad point pair")?;
                    let p1 = p1.to_finite().or(backup_p1.to_finite())?;
                    let p2 = p2.to_finite().or(backup_p2.to_finite())?;
                    overlay.push((Overlay::Line(p1, p2), size, egui::Color32::LIGHT_GREEN));
                }
                _ if m.is_flat() => {
                    let ipns = m.ipns();
                    if ipns.grade() == 1 {
                        overlay.push((
                            Overlay::Arrow(
                                vector![],
                                ipns.ipns_plane_normal().context("bad plane normal")?
                                    * ipns.ipns_plane_distance().context("bad plane normal")?,
                            ),
                            size,
                            egui::Color32::LIGHT_BLUE,
                        ));
                    }
                }
                _ => (),
            }
            Ok(())
        };
        for &shape in &self.vis {
            let _ = append_overlay(shape, 1.0);
        }
        if let Some(shape) = self.hovered {
            let _ = append_overlay(shape, 2.0);
        }
    }

    fn shape_ui(
        &mut self,
        ui: &mut egui::Ui,
        shapes: &ShapeArena<EuclideanCgaManifold>,
        shape: ShapeRef,
    ) {
        if let Err(e) = self.try_shape_ui(ui, shapes, shape) {
            ui.colored_label(egui::Color32::RED, e.to_string());
        }
    }

    fn try_shape_ui(
        &mut self,
        ui: &mut egui::Ui,
        shapes: &ShapeArena<EuclideanCgaManifold>,
        shape: ShapeRef,
    ) -> Result<()> {
        let ndim = shapes[shape.id].ndim()?;
        let space_ndim = shapes.space().ndim()?;

        ui.group(|ui| {
            let mut checkbox_label = String::new();
            checkbox_label += &format!("{}{}    ", shape.sign, shape.id);

            let manifold = &shapes[shape.id].manifold;
            let mut need_to_show_multivector = true;
            if ndim == 0 {
                if let Ok([p1, p2]) = manifold.to_point_pair() {
                    checkbox_label += &format!("point pair {p1} .. {p2}");
                    need_to_show_multivector = false;
                }
            } else if ndim == space_ndim - 1 {
                let ipns = manifold.ipns();
                if manifold.is_flat() {
                    if let (Some(n), Some(d)) =
                        (ipns.ipns_plane_normal(), ipns.ipns_plane_distance())
                    {
                        let n = n.pad(space_ndim);
                        checkbox_label += &format!("hyperplane n={n} d={d}");
                        need_to_show_multivector = false;
                    }
                } else if let (c, Some(r)) = (ipns.ipns_sphere_center(), ipns.ipns_radius()) {
                    checkbox_label += &format!("hypersphere c={c} r={r}");
                    need_to_show_multivector = false;
                }
            }
            if need_to_show_multivector {
                checkbox_label += &manifold.to_string();
            }

            let mut checkbox_state = self.vis.contains(&shape.id);
            let r = ui.checkbox(&mut checkbox_state, checkbox_label);
            if r.hovered() {
                self.hovered = Some(shape.id);
            }
            if r.changed() {
                if ui.input(|input| input.modifiers.shift) {
                    self.set_state(shape.id, checkbox_state);
                } else {
                    self.set_state_recursively(shapes, shape.id, checkbox_state);
                }
            }

            let boundary = &shapes[shape.id].boundary;
            if !boundary.is_empty() {
                ui.collapsing(format!("children of {shape}"), |ui| {
                    for mut child in boundary.iter() {
                        child.sign *= shape.sign;
                        self.shape_ui(ui, shapes, child);
                    }
                });
            }
        });

        Ok(())
    }

    fn set_state(&mut self, shape: ShapeId, state: bool) {
        if state {
            self.vis.insert(shape);
        } else {
            self.vis.remove(&shape);
        }
    }
    fn set_state_recursively(
        &mut self,
        shapes: &ShapeArena<EuclideanCgaManifold>,
        shape: ShapeId,
        state: bool,
    ) {
        self.set_state(shape, state);
        for b in shapes[shape].boundary.iter() {
            self.set_state_recursively(shapes, b.id, state);
        }
    }
}
