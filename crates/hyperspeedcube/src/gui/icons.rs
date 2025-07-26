use egui::AtomExt;

const SVG_ICON_SIZE: f32 = 12.0;

#[derive(Debug, Clone)]
pub struct CatalogIcon {
    pub icon_data: egui::ImageSource<'static>,
    pub description: &'static str,
    pub side: egui::panel::Side,
    pub color: IconColor,
}
impl CatalogIcon {
    pub fn to_image(&self, ui: &egui::Ui) -> egui::Image<'static> {
        egui::Image::from(self.icon_data.clone())
            .tint(self.color.to_color32(ui))
            .fit_to_exact_size(egui::vec2(SVG_ICON_SIZE, SVG_ICON_SIZE))
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
    ($source:literal, $description:literal, $side:ident) => {
        svg_catalog_icon!($source, $description, $side, Neutral)
    };
    ($source:literal, $description:literal, $side:ident, $color:literal) => {{
        let [r, g, b] = color_hex::color_from_hex!($color);
        let color = egui::Color32::from_rgb(r, g, b);
        svg_catalog_icon!($source, $description, $side, IconColor::Custom(color))
    }};
    ($source:literal, $description:literal, $side:ident, $color:ident) => {
        svg_catalog_icon!($source, $description, $side, IconColor::$color)
    };
    ($source:literal, $description:literal, $side:ident, $color:expr) => {
        CatalogIcon {
            icon_data: egui::include_image!(concat!("../../resources/img/", $source, ".svg")),
            description: $description,
            side: egui::panel::Side::$side,
            color: $color,
        }
    };
}

impl CatalogIcon {
    const TYPE_GENERATOR: Self = svg_catalog_icon!("cog", "Generator", Left, "#0acdf4");
    // const TYPE_GENERATED_PUZZLE: Self =
    //     svg_catalog_icon!("puzzle-cog", "Generated puzzle", Left, "#c089ff");
    const TYPE_CUBIC_PUZZLE: Self =
        svg_catalog_icon!("grid-large", "Cubic puzzle", Left, "#c089ff");
    const TYPE_HYPERCUBIC_PUZZLE: Self =
        svg_catalog_icon!("cube-outline", "Hypercubic puzzle", Left, "#c089ff");
    // const TYPE_DUOPRISM_PUZZLE: Self =
    //     svg_catalog_icon!("duoprism", "Duoprism puzzle", Left, "#c089ff");
    const TYPE_PUZZLE: Self = svg_catalog_icon!("puzzle", "Puzzle", Left, "#c089ff");
    const TYPE_SHAPE: Self = svg_catalog_icon!("pentagon", "Shape", Left, "#e1de82");
    const TYPE_SHAPE_GENERATOR: Self =
        svg_catalog_icon!("pentagon-gear", "Shape generator", Left, "#e1de82");
    const TYPE_UNKNOWN: Self = svg_catalog_icon!("help", "Missing `type` tag", Left, Error);

    const EXPERIMENTAL: Self = svg_catalog_icon!("test-tube", "Experimental", Right, "#1aeb8a");
    const BIG: Self = svg_catalog_icon!("alert", "Big/slow to generate", Right, Warn);

    const NDIM_1D: Self = svg_catalog_icon!("1d", "1D", Right);
    const NDIM_2D: Self = svg_catalog_icon!("2d", "2D", Right);
    const NDIM_3D: Self = svg_catalog_icon!("3d", "3D", Right);
    const NDIM_4D: Self = svg_catalog_icon!("4d", "4D", Right);
    const NDIM_5D: Self = svg_catalog_icon!("5d", "5D", Right);
    const NDIM_6D: Self = svg_catalog_icon!("6d", "6D", Right);
    const NDIM_7D: Self = svg_catalog_icon!("7d", "7D", Right);
    const NDIM_8D: Self = svg_catalog_icon!("8d", "8D", Right);

    pub fn icons_from_tags(tags: &hyperpuzzle::TagSet) -> Vec<CatalogIcon> {
        let mut ret = vec![];

        // Type tags
        if tags.has_present("type/generator") {
            if tags.has_present("type/shape") {
                ret.push(Self::TYPE_SHAPE_GENERATOR);
            } else {
                ret.push(Self::TYPE_GENERATOR);
            }
        } else if tags.has_present("type/puzzle") {
            if tags.has_present("shape/3d/platonic/cube") {
                ret.push(Self::TYPE_CUBIC_PUZZLE);
            } else if tags.has_present("shape/4d/platonic/hypercube") {
                ret.push(Self::TYPE_HYPERCUBIC_PUZZLE);
            // } else if tags.has_present("shape/4d/duoprism") {
            //     ret.push(Self::TYPE_DUOPRISM_PUZZLE);
            // } else if tags.has_present("generated") {
            //     ret.push(Self::TYPE_GENERATED_PUZZLE);
            } else {
                ret.push(Self::TYPE_PUZZLE);
            }
        } else if tags.has_present("type/shape") {
            ret.push(Self::TYPE_SHAPE);
        } else {
            ret.push(Self::TYPE_UNKNOWN);
        }

        // Experimental
        if tags.is_experimental() {
            ret.push(Self::EXPERIMENTAL);
        }

        // Big
        if tags.has_present("big") {
            ret.push(Self::BIG);
        }

        // Dimension tags
        for (tag, icon) in [
            ("shape/1d", Self::NDIM_1D),
            ("shape/2d", Self::NDIM_2D),
            ("shape/3d", Self::NDIM_3D),
            ("shape/4d", Self::NDIM_4D),
            ("shape/5d", Self::NDIM_5D),
            ("shape/6d", Self::NDIM_6D),
            ("shape/7d", Self::NDIM_7D),
            ("shape/8d", Self::NDIM_8D),
        ] {
            if tags.has_present(tag) {
                ret.push(icon);
            }
        }

        ret
    }
}
