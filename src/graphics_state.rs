use pathfinder_content::stroke::StrokeStyle;
use pathfinder_geometry::transform2d::Transform2F;
use pdf::object::ColorSpace;

use crate::plotter::{Fill, Plotter, Stroke};

#[derive(Copy, Debug)]
pub struct GraphicsState<'a, P: Plotter> {
    //pub transform: Transform2F,
    pub transform: Transform2F,
    pub stroke_style: StrokeStyle,

    pub fill_color: Fill,
    pub fill_color_alpha: f32,
    pub fill_paint: Option<u16>, //Option<PaintId>,
    pub stroke_color: Fill,
    pub stroke_color_alpha: f32,
    pub stroke_paint: Option<u16>, //Option<PaintId>,
    pub clip_path_id: Option<P::ClipPathId>,
    //pub clip_path: Option<ClipPath>,
    //pub clip_path_rect: Option<RectF>,
    pub fill_color_space: &'a ColorSpace,
    pub stroke_color_space: &'a ColorSpace,
    pub dash_pattern: Option<(&'a [f32], f32)>,

    pub stroke_alpha: f32,
    pub fill_alpha: f32,

    pub overprint_fill: bool,
    pub overprint_stroke: bool,
    pub overprint_mode: i32,
}

impl<'a, P: Plotter> Clone for GraphicsState<'a, P> {
    fn clone(&self) -> Self {
        GraphicsState {
            //clip_path: self.clip_path.clone(),
            .. *self
        }
    }
}

impl<'a, P: Plotter> GraphicsState<'a, P> {
    pub fn set_fill_color(&mut self, fill: Fill) {
        if fill != self.fill_color {
            self.fill_color = fill;
            self.fill_paint = None;
        }
    }
    pub fn set_fill_alpha(&mut self, alpha: f32) {
        let a = self.fill_alpha * alpha;
        if a != self.fill_color_alpha {
            self.fill_color_alpha = a;
            self.fill_paint = None;
        }
    }
    pub fn set_stroke_color(&mut self, fill: Fill) {
        if fill != self.stroke_color {
            self.stroke_color = fill;
            self.stroke_paint = None;
        }
    }
    pub fn set_stroke_alpha(&mut self, alpha: f32) {
        let a = self.stroke_alpha * alpha;
        if a != self.stroke_color_alpha {
            self.stroke_alpha = a;
            self.stroke_paint = None;
        }
    }
    pub fn stroke(&self) -> Stroke {
        Stroke {
            style: self.stroke_style,
            dash_pattern: self.dash_pattern.map(|(a, p)| (a.into(), p))
        }
    }
}
