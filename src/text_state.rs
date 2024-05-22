use std::sync::Arc;

use itertools::Itertools;

use pathfinder_canvas::{RectF, Transform2F, Vector2F};
use pdf::content::{Matrix, TextMode};

use crate::plotter::Fill;

#[derive(Clone, Debug)]
pub struct TextState {
    pub text_matrix: Transform2F, // tracks current glyph
    pub line_matrix: Transform2F, // tracks current line
    pub char_space: f32, // Character spacing
    pub word_space: f32, // Word spacing
    pub horiz_scale: f32, // Horizontal scaling
    pub leading: f32, // Leading
    //pub font_entry: Option<Arc<FontEntry>>, // Text font
    pub font_size: f32, // Text font size
    pub mode: TextMode, // Text rendering mode
    pub rise: f32, // Text rise
    pub knockout: f32, //Text knockout
}

impl TextState {
    pub fn new() -> Self {
        Self {
            text_matrix: Transform2F::default(),
            line_matrix: Transform2F::default(),
            char_space: 0.,
            word_space: 0.,
            horiz_scale: 1.,
            leading: 0.,
            //font_entry: None,
            font_size: 0.,
            mode: TextMode::Fill,
            rise: 0.,
            knockout: 0.
        }
    }
    pub fn reset_matrix(&mut self) {
        self.set_matrix(Transform2F::default());
    }
    // set text and line matrix
    pub fn set_matrix(&mut self, m: Transform2F) {
        self.text_matrix = m;
        self.line_matrix = m;
    }
    pub fn translate(&mut self, v: Vector2F) {
        let m = self.line_matrix * Transform2F::from_translation(v);
        self.set_matrix(m);
    }
    // move to the next line
    pub fn next_line(&mut self) {
        self.translate(Vector2F::new(0., -self.leading));
    }

}

#[derive(Copy, Clone, Default)]
pub struct BBox(Option<RectF>);
impl BBox {
    pub fn empty() -> Self {
        BBox(None)
    }
    pub fn add(&mut self, r2: RectF) {
        self.0 = Some(match self.0 {
            Some(r1) => r1.union_rect(r2),
            None => r2
        });
    }
    pub fn add_bbox(&mut self, bb: Self) {
        if let Some(r) = bb.0 {
            self.add(r);
        }
    }
    pub fn rect(self) -> Option<RectF> {
        self.0
    }
}
#[derive(Debug, Clone, Copy)]
pub struct TextChar {
    pub offset: usize,
    pub pos: f32,
    pub width: f32,
}
#[derive(Default)]
pub struct Span {
    pub text: String,
    pub chars: Vec<TextChar>,
    pub width: f32,
    pub bbox: BBox,
}

pub struct Part<'a> {
    pub text: &'a str,
    pub pos: f32,
    pub width: f32,
    pub offset: usize,
}

#[derive(Debug)]
pub struct TextSpan {
    // A rect with the origin at the baseline, a height of 1em and width that corresponds to the advance width.
    pub rect: RectF,

    // width in textspace units (before applying transform)
    pub width: f32,
    // Bounding box of the rendered outline
    pub bbox: Option<RectF>,
    pub font_size: f32,
    // #[debug(skip)]
    //pub font: Option<Arc<FontEntry>>,
    pub text: String,
    pub chars: Vec<TextChar>,
    pub color: Fill,
    pub alpha: f32,

    // apply this transform to a text draw in at the origin with the given width and font-size
    pub transform: Transform2F,
    pub mode: TextMode,
    pub op_nr: usize,
}
impl TextSpan {
    pub fn parts(&self) -> impl Iterator<Item=Part> + '_ {
        self.chars.iter().cloned()
            .chain(std::iter::once(TextChar { offset: self.text.len(), pos: self.width, width: 0.0 }))
            .tuple_windows()
            .map(|(a, b)| Part {
                text: &self.text[a.offset..b.offset],
                pos: a.pos,
                width: a.width,
                offset: a.offset
            })
    }
    pub fn rparts(&self) -> impl Iterator<Item=Part> + '_ {
        self.chars.iter().cloned()
            .chain(std::iter::once(TextChar { offset: self.text.len(), pos: self.width, width: 0.0 })).rev()
            .tuple_windows()
            .map(|(b, a)| Part {
                text: &self.text[a.offset..b.offset],
                pos: a.pos,
                width: a.width,
                offset: a.offset
            })
    }
}
