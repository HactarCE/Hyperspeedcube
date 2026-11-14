use hyperpuzzle::notation::{LayerFeatures, LayerPrefix};

use crate::gui::{EguiValue, ext::*};

const LAYER_DESCRIPTION_WIDTH: f32 = 50.0;

pub struct LayerMaskEdit<'a> {
    pub id: egui::Id,
    pub layers: &'a mut LayerPrefix,
}
impl egui::Widget for LayerMaskEdit<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;
        let mut r = ui
            .scope(|ui| {
                let get_default_string = || format!("{{{}}}", self.layers);

                let mut text = EguiValue::<String>::load_or_else(ui, self.id, get_default_string);

                let r = egui::TextEdit::singleline(&mut *text)
                    .desired_width(LAYER_DESCRIPTION_WIDTH)
                    .show(ui)
                    .response
                    .response;

                if r.changed() {
                    // Try to parse the new layer mask string.
                    *self.layers = hyperpuzzle::notation::parse_layer_prefix(
                        &text,
                        LayerFeatures::HYPERCUBING,
                    )
                    .unwrap_or_default();
                    changed = true;
                } else if !r.has_focus() {
                    *text = get_default_string();
                }

                r.on_hover_explanation(
                    "Layer mask string",
                    "Comma-separated list of layers or layer ranges, such as '1..3'. \
                     Negative numbers count from the other side of the puzzle. \
                     Exclamation mark prefix excludes a range.\n\
                     \n\
                     Examples:\n\
                     • {1} = outer layer\n\
                     • {2} = next layer in\n\
                     • {1,-1} = outer layer on either side\n\
                     • {1..3} = three outer layers\n\
                     • {1..-1} = whole puzzle\n\
                     • {1..-1,!3} = all except layer 3",
                );
            })
            .response;
        if changed {
            r.mark_changed();
        }
        r
    }
}
