use super::{Location, Window, WELCOME_WINDOW_WIDTH};
use crate::app::App;
use crate::gui::util::{set_widget_spacing_to_space_width, subtract_space};

const HYPERCUBERS_DISCORD_INVITE_URL: &str = "https://discord.gg/Rrw2xeB3Gb";
const HYPERCUBING_GOOGLE_GROUP_URL: &str = "https://groups.google.com/g/hypercubing";

pub const WELCOME: Window = Window {
    name: "Welcome",
    location: Location::Centered,
    fixed_width: Some(WELCOME_WINDOW_WIDTH),
    build,
    ..Window::DEFAULT
};

fn build(ui: &mut egui::Ui, app: &mut App) {
    // Adjust spacing so we don't have to add spaces manually.
    set_widget_spacing_to_space_width(ui);

    ui.horizontal_wrapped(|ui| {
        ui.label("If you're new to 4D puzzles, learn more at ");
        ui.hyperlink("https://hypercubing.xyz/");
        subtract_space(ui);
        ui.label(".");
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Also consider joining the");
        ui.hyperlink_to("Discord server", HYPERCUBERS_DISCORD_INVITE_URL);
        ui.label("and");
        ui.hyperlink_to("mailing list", HYPERCUBING_GOOGLE_GROUP_URL);
        subtract_space(ui);
        ui.label(".");
    });

    ui.label("");

    ui.horizontal_wrapped(|ui| {
        ui.label("Nearly every aspect of this program can be customized from the");
        ui.strong("Settings");
        ui.label("menu.");
    });

    ui.label("");

    egui::CollapsingHeader::new("What the heck is this?").default_open(true).show(ui, |ui| {
        ui.label("This program simulates 4-dimensional analogues of the 3D Rubik's cube. Here are some videos that can help explain:");
        ui.add(ResourceLink {
            name: "Cracking the 4D Rubik's Cube with simple 3D tricks",
            url: "https://www.youtube.com/watch?v=yhPH1369OWc",
            description: "",
        });
        ui.add(ResourceLink {
            name: "How to Solve a 4D Rubik's Cube | Beginner's Method Tutorial",
            url: "https://www.youtube.com/watch?v=h4n_QdZGXf8",
            description: "",
        });
    });

    ui.label("");

    egui::CollapsingHeader::new("Speedsolving tips")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("You can hide sets of pieces using");
                ui.strong("Tools ➡ Piece filters");
                subtract_space(ui);
                ui.label(".");
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("If you really want to go fast, consider learning keyboard controls! See");
                ui.strong("Help ➡ Keybinds reference");
                ui.label("and");
                ui.strong("Settings ➡ Puzzle keybinds");
                ui.label("to get started.");
            });
        });

    ui.label("");

    egui::CollapsingHeader::new("Other software")
        .default_open(false)
        .show(ui, |ui| {
            ui.add(ResourceLink {
                name: "Magic Cube 4D",
                url: "https://superliminal.com/cube/cube.htm",
                description: "Features macros and a wider selection of 4D puzzles",
            });
            ui.add(ResourceLink {
                name: "Magic Puzzle Ultimate",
                url: "https://superliminal.com/andrey/mpu/",
                description: "Supports nearly every regular puzzle imaginable from 3D to 7D",
            });
            ui.add(ResourceLink {
                name: "MagicTile",
                url: "http://roice3.org/magictile/",
                description: "Geometrical and topological Rubik's Cube variants",
            });
        });

    ui.label("");

    let r = ui.checkbox(
        &mut app.prefs.show_welcome_at_startup,
        "Show welcome screen at startup",
    );
    app.prefs.needs_save |= r.changed();
    ui.horizontal_wrapped(|ui| {
        ui.label("You can reopen this window from");
        ui.strong("Help ➡ Welcome");
        subtract_space(ui);
        ui.label(".");
    });
}

struct ResourceLink<'a> {
    name: &'a str,
    url: &'a str,
    description: &'a str,
}
impl egui::Widget for ResourceLink<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal_wrapped(|ui| {
            ui.label("•");
            ui.hyperlink_to(self.name, self.url);
            if !self.description.is_empty() {
                ui.label("-");
                ui.label(self.description);
            }
        })
        .response
    }
}
