use eframe::egui;
use itertools::Itertools;
use ndpuzzle::{
    math::*,
    polytope::{self, *},
    schlafli::SchlafliSymbol,
};

const MAX_NDIM: u8 = 8;

const EPSILON: f32 = 0.001;

const AXIS_NAMES: &[&str] = &["X", "Y", "Z", "W", "U", "V", "R", "S"];

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Polytope generator demo",
        options,
        Box::new(|_cc| {
            let mut dim_mappings = vec![Vector::EMPTY; MAX_NDIM as _];
            for i in 0..4 {
                dim_mappings[i] = Vector::unit(i as _);
            }

            Box::new(PolytopeDemo {
                polygons: vec![],
                ndim: 3,
                dim_mappings,

                auto_generate: false,

                schlafli: "4,3,3,3".to_string(),
                poles: vec![Vector::unit(0)],
                arrows: vec![],

                camera_rot: Matrix::EMPTY_IDENT,
                active_axes: [0, 1, 2],
                w_offset: 4.,

                error: String::new(),
            })
        }),
    );
}

#[derive(Debug)]
struct PolytopeDemo {
    polygons: Vec<Polygon>,
    ndim: u8,
    dim_mappings: Vec<Vector<f32>>,

    auto_generate: bool,

    schlafli: String,
    poles: Vec<Vector<f32>>,

    arrows: Vec<Vector<f32>>,

    camera_rot: Matrix<f32>,
    active_axes: [u8; 3],
    w_offset: f32,

    error: String,
}

impl PolytopeDemo {
    fn is_axis_flat(&self, axis: u8) -> bool {
        self.camera_rot.get(axis, axis) > 1. - 0.00001
    }
    fn flatten_axis(&mut self, axis: u8) {
        let current = self.camera_rot.col(axis);
        let target = Vector::unit(axis);
        let tm = Matrix::from_outer_product(current, &target);
        let tm = &tm - &tm.transpose();
        let m0 = &(&Matrix::ident(MAX_NDIM) + &tm)
            + &((&tm * &tm).scale(1. / (1. + current.dot(target))));
        self.camera_rot = &m0 * &self.camera_rot;
    }

    fn rotate_camera(&mut self, axis0: u8, axis1: u8, angle: f32) {
        let cangle = angle.cos();
        let sangle = angle.sin();

        let mut m0 = Matrix::ident(MAX_NDIM);
        *m0.get_mut(axis0, axis0) = cangle;
        *m0.get_mut(axis0, axis1) = sangle;
        *m0.get_mut(axis1, axis0) = sangle;
        *m0.get_mut(axis1, axis1) = -cangle;
        self.camera_rot = &m0 * &self.camera_rot;
    }
}

impl eframe::App for PolytopeDemo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::new(egui::containers::panel::Side::Right, "right").show(ctx, |ui| {
            egui::CollapsingHeader::new("View controls")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label("Active axes");
                    for active_axis in &mut self.active_axes {
                        ui.horizontal(|ui| {
                            for (i, &axis_name) in AXIS_NAMES.iter().enumerate() {
                                ui.selectable_value(active_axis, i as u8, axis_name);
                            }
                        });
                    }

                    ui.separator();

                    ui.label("Zeroed axes");
                    ui.horizontal(|ui| {
                        for (i, &axis_name) in AXIS_NAMES.iter().enumerate() {
                            if ui
                                .selectable_label(self.is_axis_flat(i as u8), axis_name)
                                .clicked()
                            {
                                self.flatten_axis(i as u8);
                            }
                        }
                    });

                    ui.separator();

                    ui.add_enabled_ui(
                        !self.camera_rot.approx_eq(&Matrix::EMPTY_IDENT, EPSILON),
                        |ui| {
                            if ui.button("Reset camera rotation").clicked() {
                                self.camera_rot = Matrix::EMPTY_IDENT;
                            }
                        },
                    );

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.w_offset)
                                .speed(0.01)
                                .fixed_decimals(1),
                        );
                        ui.label("W offset")
                    });
                });

            ui.separator();
            ui.collapsing("Polytope specification", |ui| {
                ui.label("Schlafli symbol");
                ui.text_edit_singleline(&mut self.schlafli);
                let xs = self
                    .schlafli
                    .split(',')
                    .map(|s| s.trim().parse().unwrap_or(0))
                    .collect_vec();
                self.ndim = xs.len() as u8;

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("+").clicked() {
                        self.poles.push(Vector::EMPTY);
                    }
                    if ui.button("-").clicked() && self.poles.len() > 1 {
                        self.poles.pop();
                    }
                    ui.label("Base facets");
                });
                for p in &mut self.poles {
                    vector_edit(ui, p, self.ndim);
                }

                ui.separator();

                if ui.button("Generate!").clicked() || self.auto_generate {
                    if xs.iter().any(|&x| x <= 1) {
                        self.error = "bad schlafli symbol".to_string();
                    } else {
                        let schlafli = SchlafliSymbol::from_indices(xs);
                        self.ndim = schlafli.ndim();
                        self.arrows = schlafli.mirrors().iter().map(|v| v.0.clone()).collect();
                        let m = Matrix::from_cols(schlafli.mirrors().iter().rev().map(|v| &v.0))
                            .inverse()
                            .unwrap_or(Matrix::EMPTY_IDENT) // TODO: isn't really right
                            .transpose();
                        let group = schlafli.generators();
                        let poles = self
                            .poles
                            .iter()
                            .map(|v| m.transform(v.clone().resize(self.ndim)))
                            .collect::<Vec<_>>();
                        self.arrows.extend_from_slice(&poles);
                        self.error = String::new();
                        match polytope::generate_polytope(self.ndim, &group, &poles) {
                            Ok(polys) => self.polygons = polys,
                            Err(e) => self.error = e.to_string(),
                        }
                    }
                }
                ui.checkbox(&mut self.auto_generate, "Auto generate");
                ui.colored_label(egui::Color32::RED, &self.error);
            });

            ui.separator();
            for (v, &axis_name) in self.dim_mappings.iter_mut().zip(AXIS_NAMES) {
                ui.horizontal(|ui| {
                    ui.scope(|ui| {
                        ui.set_min_width(15.0);
                        ui.label(axis_name);
                    });
                    if ui.button("N").clicked() {
                        if v.dot(&*v) != 0.0 {
                            *v = &*v * (1.0 / v.dot(&*v).sqrt());
                        }
                    }
                    vector_edit(ui, v, 4);
                });
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Polytope generator demo");
            let r = egui::plot::Plot::new("polygon_plot")
                .data_aspect(1.0)
                .allow_boxed_zoom(false)
                .show(ui, |plot_ui| {
                    let ndrot = &Matrix::from_cols(self.dim_mappings.clone()) * &self.camera_rot;
                    // let rot = cgmath::Matrix3::from_angle_x(cgmath::Rad(self.pitch))
                    //     * cgmath::Matrix3::from_angle_y(cgmath::Rad(self.yaw));
                    for (i, p) in self.polygons.iter().enumerate() {
                        plot_ui.polygon(
                            egui::plot::Polygon::new(egui::plot::Values::from_values_iter(
                                p.verts
                                    .iter()
                                    .map(|p| {
                                        let mut v = ndrot.transform(p);
                                        let w = v[3] + self.w_offset;
                                        v = v / w;
                                        cgmath::point3(v[0], v[1], v[2])
                                    })
                                    .map(|xy| egui::plot::Value::new(xy.x, xy.y)),
                            ))
                            .name(i),
                        );
                    }
                    plot_ui.arrows(egui::plot::Arrows::new(
                        egui::plot::Values::from_values_iter(
                            vec![egui::plot::Value::new(0, 0); self.arrows.len()].into_iter(),
                        ),
                        egui::plot::Values::from_values_iter(
                            self.arrows
                                .iter()
                                .map(|p| {
                                    let mut v = ndrot.transform(p);
                                    let w = v[3] + self.w_offset;
                                    v = v / w;
                                    cgmath::point3(v[0], v[1], v[2])
                                })
                                .map(|xy| egui::plot::Value::new(xy.x, xy.y)),
                        ),
                    ))
                });
            if r.response.dragged_by(egui::PointerButton::Secondary) {
                let egui::Vec2 { x, y } = r.response.drag_delta();
                let dx = x / 100.;
                let dy = y / 100.;

                let [a0, a1, a2] = self.active_axes;

                self.rotate_camera(a0, a2, dx);
                self.rotate_camera(a1, a2, dy);
            }
        });
    }
}

fn vector_edit(ui: &mut egui::Ui, v: &mut Vector<f32>, ndim: u8) {
    *v = v.clone().resize(ndim);
    ui.horizontal(|ui| {
        for i in 0..ndim {
            ui.add(
                egui::DragValue::new(&mut v[i])
                    .speed(0.01)
                    .fixed_decimals(1),
            )
            .on_hover_text(format!("Dim {i}"));
        }
    });
}
