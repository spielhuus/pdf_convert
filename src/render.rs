use pathfinder_content::{
    fill::FillRule,
    outline::{Contour, Outline},
    stroke::StrokeStyle,
};
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F, vector::Vector2F};
use pdf::{
    content::{Cmyk, Matrix, Op, Point, Rect, Rgb, Winding},
    object::{ColorSpace, Page, Resolve, Resources},
    PdfError,
};

use crate::{
    graphics_state::GraphicsState,
    plotter::{BlendMode, DrawMode, Fill, FillMode, Plotter},
    text_state::TextState,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ClipPathId(pub u32);

trait Cvt {
    type Out;
    fn cvt(self) -> Self::Out;
}
impl Cvt for Point {
    type Out = Vector2F;
    fn cvt(self) -> Self::Out {
        Vector2F::new(self.x, self.y)
    }
}
impl Cvt for Matrix {
    type Out = Transform2F;
    fn cvt(self) -> Self::Out {
        let Matrix { a, b, c, d, e, f } = self;
        Transform2F::row_major(a, c, e, b, d, f)
    }
}
impl Cvt for Rect {
    type Out = RectF;
    fn cvt(self) -> Self::Out {
        RectF::new(
            Vector2F::new(self.x, self.y),
            Vector2F::new(self.width, self.height),
        )
    }
}
impl Cvt for Winding {
    type Out = FillRule;
    fn cvt(self) -> Self::Out {
        match self {
            Winding::NonZero => FillRule::Winding,
            Winding::EvenOdd => FillRule::EvenOdd,
        }
    }
}
impl Cvt for Rgb {
    type Out = (f32, f32, f32);
    fn cvt(self) -> Self::Out {
        let Rgb { red, green, blue } = self;
        (red, green, blue)
    }
}
impl Cvt for Cmyk {
    type Out = (f32, f32, f32, f32);
    fn cvt(self) -> Self::Out {
        let Cmyk {
            cyan,
            magenta,
            yellow,
            key,
        } = self;
        (cyan, magenta, yellow, key)
    }
}

enum PathTokens {
    MoveTo { x: f32, y: f32 },
    LineTo { x: f32, y: f32 },
}

pub struct RenderState<'a, R: Resolve, P: Plotter> {
    graphics_state: GraphicsState<'a, P>,
    text_state: TextState,
    //text_state: TextState,
    plotter: &'a mut P,
    current_outline: Outline,
    current_contour: Contour,
    resolve: &'a R,
    resources: &'a Resources,
    transform: Transform2F,
    //stack: Vec<(GraphicsState<'a, B>, TextState)>,
    //data: Vec<Command>,
    path: Vec<PathTokens>,
    stack: Vec<(GraphicsState<'a, P>, TextState)>,
}

impl<'a, R: Resolve, P: Plotter> RenderState<'a, R, P> {
    pub fn new(
        plotter: &'a mut P,
        resolve: &'a mut R,
        resources: &'a Resources,
        transform: Transform2F,
    ) -> Self {
        Self {
            graphics_state: GraphicsState {
                transform,
                stroke_style: StrokeStyle::default(),
                fill_color: Fill::black(),
                fill_color_alpha: 1.0,
                fill_paint: None,
                stroke_color: Fill::black(),
                stroke_color_alpha: 1.0,
                stroke_paint: None,
                clip_path_id: None,
                //clip_path: None,
                //clip_path_rect: None,
                fill_color_space: &ColorSpace::DeviceRGB,
                stroke_color_space: &ColorSpace::DeviceRGB,
                dash_pattern: None,
                stroke_alpha: 1.0,
                fill_alpha: 1.0,
                overprint_fill: false,
                overprint_stroke: false,
                overprint_mode: 0,
            },
            plotter,
            resolve,
            resources,
            transform,
            path: vec![],
            text_state: TextState::new(),
            //resolve,
            //resources,
            stack: vec![],
            //data: vec![],
            current_outline: Outline::new(),
            current_contour: Contour::new(),
        }
    }

    //fn line_to(&mut self, x: f32, y: f32) {
    //    self.path.push(PathTokens::LineTo { x, y });
    //}
    //fn move_to(&mut self, x: f32, y: f32) {
    //    self.path.push(PathTokens::MoveTo { x, y });
    //}
    //fn stroke(&mut self) {}
    //fn transform(&mut self, matrix: Transform2F) {}
    //fn close(&mut self) {}
    fn flush(&mut self) {
        if !self.current_contour.is_empty() {
            self.current_outline
                .push_contour(self.current_contour.clone());
            self.current_contour.clear();
        }
    }
    fn blend_mode_stroke(&self) -> BlendMode {
        if self.graphics_state.overprint_stroke {
            BlendMode::Darken
        } else {
            BlendMode::Overlay
        }
    }
    fn draw(&mut self, mode: &DrawMode, fill_rule: FillRule) {
        self.flush();
        self.plotter.draw(&self.current_outline, mode, fill_rule, self.graphics_state.transform, self.graphics_state.clip_path_id);
        self.current_outline.clear();
    }
    pub fn render(&mut self, page: &Page) -> Result<(), PdfError> {
        let contents = pdf::try_opt!(page.contents.as_ref());
        let ops = contents.operations(self.resolve)?;

        for (i, op) in ops.iter().enumerate() {
            //println!("op {}: {:?}", i, op);
            match op {
                Op::BeginMarkedContent { tag, properties } => {}
                Op::EndMarkedContent => {}
                Op::MarkedContentPoint { tag, properties } => {}
                Op::Close => {
                    self.current_contour.close();
                }
                Op::MoveTo { p } => {
                    self.flush();
                    self.current_contour.push_endpoint(p.cvt());
                }
                Op::LineTo { p } => {
                    self.current_contour.push_endpoint(p.cvt());
                }
                Op::CurveTo { c1, c2, p } => {
                    self.current_contour.push_cubic(c1.cvt(), c2.cvt(), p.cvt());
                }
                Op::Rect { rect } => {
                    self.flush();
                    self.current_outline
                        .push_contour(Contour::from_rect(rect.cvt()));
                }
                Op::EndPath => {
                    self.current_contour.clear();
                    self.current_outline.clear();
                }

                Op::Stroke => {
                    self.draw(&DrawMode::Stroke {
                        stroke: FillMode {
                            color: self.graphics_state.stroke_color,
                            alpha: self.graphics_state.stroke_color_alpha,
                            mode: self.blend_mode_stroke(),
                        },
                        stroke_mode: self.graphics_state.stroke()},
                        FillRule::Winding
                    );
                },
                Op::FillAndStroke { winding } => {} //{},
                Op::Fill { winding } => {}          //{},
                Op::Shade { name } => {}
                Op::Clip { winding } => {} //{},
                Op::Save => {
                    self.stack
                        .push((self.graphics_state.clone(), self.text_state.clone()));
                }
                pdf::content::Op::Restore => {
                    let (g, t) = self
                        .stack
                        .pop()
                        .ok_or_else(|| pdf::error::PdfError::Other {
                            msg: "graphcs stack is empty".into(),
                        })?;
                    self.graphics_state = g;
                    self.text_state = t;
                }
                pdf::content::Op::Transform { matrix } => {
                    let Matrix { a, b, c, d, e, f } = matrix;
                    let matrix = Transform2F::row_major(*a, *c, *e, *b, *d, *f);
                    self.graphics_state.transform = self.graphics_state.transform * matrix;
                }
                pdf::content::Op::LineWidth { width } => {} //{},
                pdf::content::Op::Dash { pattern, phase } => {}
                pdf::content::Op::LineJoin { join } => {}
                pdf::content::Op::LineCap { cap } => {}
                pdf::content::Op::MiterLimit { limit } => {}
                pdf::content::Op::Flatness { tolerance } => {}
                pdf::content::Op::GraphicsState { name } => {},
                pdf::content::Op::StrokeColor { color } => {} //{},
                pdf::content::Op::FillColor { color } => {}   //{},
                pdf::content::Op::FillColorSpace { name } => {} //{},
                pdf::content::Op::StrokeColorSpace { name } => {} //{},
                pdf::content::Op::RenderingIntent { intent } => {}
                pdf::content::Op::BeginText => {}
                pdf::content::Op::EndText => {}
                pdf::content::Op::CharSpacing { char_space } => {}
                pdf::content::Op::WordSpacing { word_space } => {}
                pdf::content::Op::TextScaling { horiz_scale } => {}
                pdf::content::Op::Leading { leading } => {}
                pdf::content::Op::TextFont { name, size } => {}
                pdf::content::Op::TextRenderMode { mode } => {}
                pdf::content::Op::TextRise { rise } => {}
                pdf::content::Op::MoveTextPosition { translation } => {}
                pdf::content::Op::SetTextMatrix { matrix } => {}
                pdf::content::Op::TextNewline => {}
                pdf::content::Op::TextDraw { text } => {}
                pdf::content::Op::TextDrawAdjusted { array } => {}
                pdf::content::Op::XObject { name } => {}
                pdf::content::Op::InlineImage { image } => {}
            }
            //if let Some(path) = renderstate.draw_op(op, i)? {
            //    document = document.add(path);
            //}
        }

        Ok(())
    }
}
