use egui::AtomExt;
use hyperpuzzle::TagValue;

const SVG_ICON_SIZE: f32 = 12.0;

#[derive(Debug, Clone)]
pub struct CatalogIcon {
    pub icon_data: egui::ImageSource<'static>,
    pub icon_data_dark_mode: Option<egui::ImageSource<'static>>,
    pub description: &'static str,
    pub side: egui::panel::Side,
    pub color: IconColor,
}
impl CatalogIcon {
    pub fn to_image(&self, ui: &egui::Ui) -> egui::Image<'static> {
        let icon_data = if ui.visuals().dark_mode {
            self.icon_data_dark_mode.as_ref().unwrap_or(&self.icon_data)
        } else {
            &self.icon_data
        };
        egui::Image::from(icon_data.clone())
            .tint(self.color.to_color32(ui))
            .fit_to_exact_size(egui::vec2(SVG_ICON_SIZE * 1.5, SVG_ICON_SIZE * 1.5))
    }
    pub fn to_atom(&self, ui: &egui::Ui) -> egui::Atom<'static> {
        self.to_image(ui).into()
    }
    pub fn add_to(&self, ui: &egui::Ui, atoms: &mut egui::Atoms<'static>) {
        let atom = self.to_atom(ui);
        match self.side {
            egui::panel::Side::Left => atoms.push_left(atom),
            egui::panel::Side::Right => atoms.push_right(atom),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IconColor {
    #[default]
    Neutral,
    Warn,
    Error,
    Custom(egui::Color32),
}
impl From<egui::Color32> for IconColor {
    fn from(value: egui::Color32) -> Self {
        IconColor::Custom(value)
    }
}
impl IconColor {
    pub fn to_color32(self, ui: &egui::Ui) -> egui::Color32 {
        match self {
            IconColor::Neutral => ui.visuals().strong_text_color(),
            IconColor::Warn => ui.visuals().warn_fg_color,
            IconColor::Error => ui.visuals().error_fg_color,
            IconColor::Custom(c) => c,
        }
    }
}

macro_rules! svg_catalog_icon {
    ($source:tt, $description:literal, $side:ident) => {
        svg_catalog_icon!($source, $description, $side, Neutral)
    };
    ($source:tt, $description:literal, $side:ident, $color:literal) => {{
        let color = egui::hex_color!($color);
        svg_catalog_icon!($source, $description, $side, IconColor::Custom(color))
    }};
    ($source:tt, $description:literal, $side:ident, $color:ident) => {
        svg_catalog_icon!($source, $description, $side, IconColor::$color)
    };
    ($source:tt, $description:literal, $side:ident, $color:expr) => {{
        let (light, dark) = svg_catalog_icon!(@sources $source);
        CatalogIcon {
            icon_data: light,
            icon_data_dark_mode: dark,
            description: $description,
            side: egui::panel::Side::$side,
            color: $color,
        }
    }};
    (@sources $source:literal) => {
        (svg_catalog_icon!(@single_source $source), None)
    };
    (@sources ($light_mode_source:literal, $dark_mode_source:literal)) => {
        (
            svg_catalog_icon!(@single_source $light_mode_source),
            Some(svg_catalog_icon!(@single_source $dark_mode_source)),
        )
    };
    (@single_source $source:literal) => {
        egui::include_image!(concat!("../../resources/img/catalog/", $source, ".svg"))
    }
}

impl CatalogIcon {
    const TYPE_PUZZLE_GENERATOR: Self = svg_catalog_icon!("type/cog", "Generator", Left, "#0acdf4");
    const TYPE_PUZZLE: Self = svg_catalog_icon!("type/puzzle", "Puzzle", Left, "#c089ff");
    const TYPE_GENERATED_PUZZLE: Self =
        svg_catalog_icon!("type/puzzle-cog", "Puzzle", Left, "#c089ff");
    const TYPE_SHAPE: Self = svg_catalog_icon!("type/pentagon", "Shape", Left, "#cdc72a");
    const TYPE_GENERATED_SHAPE: Self =
        svg_catalog_icon!("type/pentagon-cog", "Generated shape", Left, "#cdc72a");
    const TYPE_SHAPE_GENERATOR: Self =
        svg_catalog_icon!("type/cog", "Shape generator", Left, "#eaa560");
    const TYPE_UNKNOWN: Self = svg_catalog_icon!("type/help", "Missing `type` tag", Left, Error);

    const EXPERIMENTAL: Self = svg_catalog_icon!("test-tube", "Experimental", Right, "#12c06f");
    const BIG: Self = svg_catalog_icon!("alert", "Big/slow to generate", Right, Warn);

    const NDIM_1D: Self = svg_catalog_icon!("ndim/1d", "1D", Left);
    const NDIM_2D: Self = svg_catalog_icon!("ndim/2d", "2D", Left, "#cdc72a"); // yellow (like D)
    const NDIM_3D: Self = svg_catalog_icon!("ndim/3d", "3D", Left, "#12c06f"); // green (like F)
    const NDIM_4D: Self = svg_catalog_icon!("ndim/4d", "4D", Left, "#c089ff"); // pink (like O)
    const NDIM_5D: Self = svg_catalog_icon!("ndim/5d", "5D", Left, "#00a9cb"); // blue
    const NDIM_6D: Self = svg_catalog_icon!("ndim/6d", "6D", Left, "#da811a"); // orange
    const NDIM_7D: Self = svg_catalog_icon!("ndim/7d", "7D", Left, "#d22e2e"); // red
    const NDIM_8D: Self = svg_catalog_icon!("ndim/8d", "8D", Left);
    #[rustfmt::skip]
    const NDIM_ND: Self = svg_catalog_icon!(("ndim/nd_light", "ndim/nd_dark"), "Dimension-generic", Left, "#ffffff"); // rainbow
    const NDIM_UNKNOWN: Self = svg_catalog_icon!("ndim/unknown", "Unknown dimension", Left);

    pub fn icons_from_tags(tags: &hyperpuzzle::TagSet) -> Vec<CatalogIcon> {
        let mut ret = vec![];

        // Dimension
        ret.push(match tags.get("ndim") {
            Some(TagValue::Int(1)) => Self::NDIM_1D,
            Some(TagValue::Int(2)) => Self::NDIM_2D,
            Some(TagValue::Int(3)) => Self::NDIM_3D,
            Some(TagValue::Int(4)) => Self::NDIM_4D,
            Some(TagValue::Int(5)) => Self::NDIM_5D,
            Some(TagValue::Int(6)) => Self::NDIM_6D,
            Some(TagValue::Int(7)) => Self::NDIM_7D,
            Some(TagValue::Int(8)) => Self::NDIM_8D,
            _ => {
                if tags.has_present("ndim/generic") {
                    Self::NDIM_ND
                } else {
                    Self::NDIM_UNKNOWN
                }
            }
        });

        // Type
        let is_shape = tags.has_present("type/shape");
        let is_puzzle = tags.has_present("type/puzzle");
        let is_generator = tags.has_present("type/generator");
        let is_generated = tags.has_present("generated");
        ret.push(match (is_shape, is_puzzle) {
            (true, _) if is_generator => Self::TYPE_SHAPE_GENERATOR,
            (true, _) if is_generated => Self::TYPE_GENERATED_SHAPE,
            (true, _) => Self::TYPE_SHAPE,
            (_, _) if is_generator => Self::TYPE_PUZZLE_GENERATOR,
            (_, true) if is_generated => Self::TYPE_GENERATED_PUZZLE,
            (_, true) => Self::TYPE_PUZZLE,
            (false, false) => Self::TYPE_UNKNOWN,
        });

        // Experimental
        if tags.is_experimental() {
            ret.push(Self::EXPERIMENTAL);
        }

        // Big
        if tags.has_present("big") {
            ret.push(Self::BIG);
        }

        ret
    }
}
