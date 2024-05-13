use pathfinder_content::{fill::FillRule, outline::Outline, stroke::StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pdf::object::{Pattern, Ref};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Fill {
    Solid(f32, f32, f32),
    Pattern(Ref<Pattern>),
}
impl Fill {
    pub fn black() -> Self {
        Fill::Solid(0., 0., 0.)
    }
}

pub struct FillMode {
    pub color: Fill,
    pub alpha: f32,
    pub mode: BlendMode,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum BlendMode {
    Overlay,
    Darken
}

pub enum DrawMode {
    Fill { fill: FillMode },
    Stroke { stroke: FillMode, stroke_mode: Stroke },
    FillStroke { fill: FillMode, stroke: FillMode, stroke_mode: Stroke },
}

#[derive(Clone, Debug)]
pub struct Stroke {
    pub dash_pattern: Option<(Vec<f32>, f32)>,
    pub style: StrokeStyle,
}

pub trait Plotter {
    type ClipPathId: Copy;

   fn draw(&mut self, outline: &Outline, mode: &DrawMode, fill_rule: FillRule, transform: Transform2F, clip: Option<Self::ClipPathId>);
}
