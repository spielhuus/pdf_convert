use std::{fs::File, io::BufWriter, path::PathBuf};

use pathfinder_color::{ColorF, ColorU};
use pathfinder_content::{dash::OutlineDash, fill::FillRule, outline::Outline, stroke::OutlineStrokeToFill};
use pathfinder_export::{Export, FileFormat};
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F};
use pathfinder_renderer::{paint::{Paint, PaintId}, scene::{ClipPathId, DrawPath, Scene}};

use crate::plotter::{BlendMode, DrawMode, Fill, Plotter};

fn blend_mode(mode: BlendMode) -> pathfinder_content::effects::BlendMode {
    match mode {
        BlendMode::Darken => pathfinder_content::effects::BlendMode::Multiply,
        BlendMode::Overlay => pathfinder_content::effects::BlendMode::Overlay,
    }
}

pub struct VectorPlotter {
    scene: Scene,
}

impl VectorPlotter {
    pub fn new(view_box: RectF) -> Self {
        let mut scene = Scene::new();
        scene.set_view_box(view_box);
        let white = scene.push_paint(&Paint::from_color(ColorU::white()));
        scene.push_draw_path(DrawPath::new(Outline::from_rect(view_box), white));
        Self {
            scene,
        }
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
    pub fn write(&mut self, file: PathBuf) {
        let mut writer = BufWriter::new(File::create(&file).unwrap());
        let format = match file.extension().and_then(|s| s.to_str()) {
            Some("pdf") => FileFormat::PDF,
            Some("ps") => FileFormat::PS,
            Some("svg") => FileFormat::SVG,
            _ => panic!("output filename must have .ps or .pdf extension")
        };
       self.scene.export(&mut writer, format).unwrap();
    }
}

impl Plotter for VectorPlotter {
    type ClipPathId = ClipPathId;
    fn draw(&mut self, outline: &Outline, mode: &DrawMode, fill_rule: FillRule, transform: Transform2F, clip: Option<Self::ClipPathId>) {
        match mode {
            DrawMode::Fill { fill } | DrawMode::FillStroke {fill, .. } => {
                let paint = self.paint(fill.color, fill.alpha);
                let mut draw_path = DrawPath::new(outline.clone().transformed(&transform), paint);
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
                        let dashed = OutlineDash::new(outline, pat, phase).into_outline();
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
                let mut draw_path = DrawPath::new(contour.transformed(&transform), paint);
                draw_path.set_clip_path(clip);
                draw_path.set_fill_rule(fill_rule);

            draw_path.set_blend_mode(blend_mode(stroke.mode));
                self.scene.push_draw_path(draw_path);
            }
            _ => {}
        }
    }
}

