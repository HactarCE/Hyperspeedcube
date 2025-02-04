use crate::commands::LayerMaskDesc;
use crate::gui::ext::*;

const LAYER_DESCRIPTION_WIDTH: f32 = 50.0;

pub struct LayerMaskEdit<'a> {
    pub id: egui::Id,
    pub layers: &'a mut LayerMaskDesc,
}
impl egui::Widget for LayerMaskEdit<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;
        let mut r = ui
            .scope(|ui| {
                let text_id = self.id.with("layer_text");

                let default_string = format!("{{{}}}", self.layers);

                let mut text: String = ui.data(|data| {
                    data.get_temp(text_id)
                        .unwrap_or_else(|| default_string.clone())
                });

                let r = egui::TextEdit::singleline(&mut text)
                    .desired_width(LAYER_DESCRIPTION_WIDTH)
                    .show(ui)
                    .response;

                if r.changed() {
                    // Try to parse the new layer mask string.
                    *self.layers = text
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .parse()
                        .unwrap_or_default();
                    changed = true;
                } else if !r.has_focus() {
                    text = default_string;
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

                ui.data_mut(|data| data.insert_temp(text_id, text));
            })
            .response;
        if changed {
            r.mark_changed();
        }
        r
    }
}
