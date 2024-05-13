use std::sync::Arc;

use crate::{graphics_state::Fill, plotter::{Backend, DrawMode}, scene::Scene};

use pathfinder_color::{ColorF, ColorU};
use pathfinder_content::{dash::OutlineDash, effects::BlendMode, fill::FillRule, gradient::Gradient, outline::Outline, pattern::Pattern, stroke::OutlineStrokeToFill};
use pathfinder_geometry::{
    vector::Vector2F,
    rect::RectF, transform2d::Transform2F,
};
use pdf::object::{ImageXObject, Ref, Resolve, Resources};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PaintId(pub u16);

#[derive(Clone, Debug)]
pub struct DrawPath {
    /// The actual vector path outline.
    pub outline: Outline,
    /// The ID of the paint that specifies how to fill the interior of this outline.
    pub paint: PaintId,
    /// The ID of an optional clip path that will be used to clip this path.
    pub clip_path: Option<ClipPathId>,
    /// How to fill this path (winding or even-odd).
    pub fill_rule: FillRule,
    /// How to blend this path with everything below it.
    pub blend_mode: BlendMode,
    /// The name of this path, for debugging.
    ///
    /// Pass the empty string (which does not allocate) if debugging is not needed.
    pub name: String,
}

impl DrawPath {
    /// Creates a new draw path with the given outline and paint.
    ///
    /// Initially, there is no clip path, the fill rule is set to winding, the blend mode is set to
    /// source-over, and the path has no name.
    #[inline]
    pub fn new(outline: Outline, paint: PaintId) -> DrawPath {
        DrawPath {
            outline,
            paint,
            clip_path: None,
            fill_rule: FillRule::Winding,
            blend_mode: BlendMode::SrcOver,
            name: String::new(),
        }
    }

    /// Returns the outline of this path, which defines its vector commands.
    #[inline]
    pub fn outline(&self) -> &Outline {
        &self.outline
    }

    #[inline]
    pub(crate) fn clip_path(&self) -> Option<ClipPathId> {
        self.clip_path
    }

    /// Sets a previously-defined clip path that will be used to limit the filled region of this
    /// path.
    ///
    /// Clip paths are defined in world space, not relative to the bounds of this path.
    #[inline]
    pub fn set_clip_path(&mut self, new_clip_path: Option<ClipPathId>) {
        self.clip_path = new_clip_path
    }

    #[inline]
    pub(crate) fn paint(&self) -> PaintId {
        self.paint
    }

    #[inline]
    pub(crate) fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    /// Sets the fill rule: even-odd or winding.
    #[inline]
    pub fn set_fill_rule(&mut self, new_fill_rule: FillRule) {
        self.fill_rule = new_fill_rule
    }

    #[inline]
    pub(crate) fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    /// Sets the blend mode, which specifies how this path will be composited with content
    /// underneath it.
    #[inline]
    pub fn set_blend_mode(&mut self, new_blend_mode: BlendMode) {
        self.blend_mode = new_blend_mode
    }

    /// Assigns a name to this path, for debugging.
    #[inline]
    pub fn set_name(&mut self, new_name: String) {
        self.name = new_name
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Paint {
    base_color: ColorU,
    overlay: Option<PaintOverlay>,
}

impl Paint {
    #[inline]
    pub fn black() -> Paint {
        Paint::from_color(ColorU::black())
    }
    #[inline]
    pub fn from_color(color: ColorU) -> Paint {
        Paint { base_color: color, overlay: None }
    }
}


#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PaintOverlay {
    composite_op: PaintCompositeOp,
    contents: PaintContents,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum PaintContents {
    /// A gradient, either linear or radial.
    Gradient(Gradient),
    /// A raster image pattern.
    Pattern(Pattern),
}

//impl Debug for PaintContents {
//    fn fmt(&self, formatter: &mut Fomatter) -> fmt::Result {
//        match *self {
//            PaintContents::Gradient(ref gradient) => gradient.fmt(formatter),
//            PaintContents::Pattern(ref pattern) => pattern.fmt(formatter),
//        }
//    }
//}

/// The ID of a gradient, unique to a scene.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GradientId(pub u32);

/// How an overlay is to be composited over a base color.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PaintCompositeOp {
    /// The source that overlaps the destination, replaces the destination.
    SrcIn,
    /// Destination which overlaps the source, replaces the source.
    DestIn,
}
#[derive(Copy, Clone, Debug)]
pub struct ClipPathId(pub usize);

pub struct Png {
    scene: Scene,
}

impl Png {
    pub fn new() -> Self {
        Self{
            scene: Scene::new(),
        }
    }
    pub fn finish(self) -> Scene  {
        self.scene
    }
    fn paint(&mut self, fill: Fill, alpha: f32) -> PaintId {
        let paint = match fill {
            Fill::Solid(r, g, b) => Paint::from_color(ColorF::new(r, g, b, alpha).to_u8()),
            Fill::Pattern(_) => {
                Paint::black()
            }
        };
        self.scene.push_paint(&paint)
    }
}

impl Backend for Png {
    type ClipPathId = ClipPathId;

    fn create_clip_path(&mut self, path: Outline, fill_rule: FillRule, parent: Option<Self::ClipPathId>) -> Self::ClipPathId {
        todo!()
    }

    fn draw(&mut self, outline: &Outline, mode: &DrawMode, fill_rule: FillRule, transform: Transform2F, clip: Option<Self::ClipPathId>) {
        match mode {
            DrawMode::Fill { fill } | DrawMode::FillStroke {fill, .. } => {
                let paint = self.paint(fill.color, fill.alpha);
                let mut outline = outline.clone();
                outline.transform(&transform);
                let mut draw_path = DrawPath::new(outline, paint);
                draw_path.set_clip_path(clip);
                draw_path.set_fill_rule(fill_rule);
                draw_path.set_blend_mode(blend_mode(fill.mode));
                self.scene.push_draw_path(draw_path);
            }
            _ => {}
        }
        match mode {
            DrawMode::Stroke { stroke, stroke_mode }| DrawMode::FillStroke { stroke, stroke_mode, .. } => {
                let paint = self.paint(stroke.color, stroke.alpha);
                let contour = match stroke_mode.dash_pattern {
                    Some((ref pat, phase)) => {
                        let dashed = OutlineDash::new(outline, &*pat, phase).into_outline();
                        let mut stroke = OutlineStrokeToFill::new(&dashed, stroke_mode.style);
                        stroke.offset();
                        stroke.into_outline()
                    }
                    None => {
                        let mut stroke = OutlineStrokeToFill::new(outline, stroke_mode.style);
                        stroke.offset();
                        stroke.into_outline()
                    }
                };
                let mut contour = outline.clone();
                contour.transform(&transform);
                let mut draw_path = DrawPath::new(contour, paint);
                draw_path.set_clip_path(clip);
                draw_path.set_fill_rule(fill_rule);

            draw_path.set_blend_mode(blend_mode(stroke.mode));
                self.scene.push_draw_path(draw_path);
            }
            _ => {}
        }
    }

    fn set_view_box(&mut self, r: pathfinder_geometry::rect::RectF) {
        println!("view box");
    }

    fn draw_image(&mut self, xref: Ref<pdf::object::XObject>, im: &ImageXObject, resources: &Resources, transform: Transform2F, mode: crate::plotter::BlendMode, clip: Option<Self::ClipPathId>, resolve: &impl pdf::object::Resolve) {
        println!("draw image");
    }

    fn draw_inline_image(&mut self, im: &Arc<ImageXObject>, resources: &Resources, transform: Transform2F, mode: crate::plotter::BlendMode, clip: Option<Self::ClipPathId>, resolve: &impl Resolve) {
        println!("draw inline");
    }

    fn add_text(&mut self, span: crate::plotter::TextSpan, clip: Option<Self::ClipPathId>) {
        println!("add text");
    }
}

fn blend_mode(mode: crate::plotter::BlendMode) -> pathfinder_content::effects::BlendMode {
    match mode {
        crate::plotter::BlendMode::Darken => pathfinder_content::effects::BlendMode::Multiply,
        crate::plotter::BlendMode::Overlay => pathfinder_content::effects::BlendMode::Overlay,
    }
}
