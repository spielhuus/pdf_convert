use std::fs::OpenOptions;
use std::io::Write;
use std::path::{self, Path};
use std::{fs::File, io::BufWriter, path::PathBuf};

use gl::types::GLvoid;
use glutin::api::egl::device::Device;
use glutin::api::egl::display::Display;
use glutin::config::{ConfigSurfaceTypes, ConfigTemplate, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder};
use glutin::prelude::*;

use pathfinder_color::{ColorF, ColorU};
use pathfinder_content::{dash::OutlineDash, fill::FillRule, outline::Outline, stroke::OutlineStrokeToFill};
use pathfinder_export::{Export, FileFormat};
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F};
use pathfinder_renderer::{paint::{Paint, PaintId}, scene::{ClipPathId, DrawPath, Scene}};

use euclid::default::Size2D;
use pathfinder_canvas::{Canvas, CanvasFontContext, Path2D};
use pathfinder_geometry::vector::{vec2f, vec2i};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererMode, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::embedded::EmbeddedResourceLoader;

use crate::plotter::{BlendMode, DrawMode, Fill, Plotter};

fn blend_mode(mode: BlendMode) -> pathfinder_content::effects::BlendMode {
    match mode {
        BlendMode::Darken => pathfinder_content::effects::BlendMode::Multiply,
        BlendMode::Overlay => pathfinder_content::effects::BlendMode::Overlay,
    }
}

pub struct PngPlotter {
    scene: Scene,
}

impl PngPlotter {
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
        render(&mut self.scene, file);
    }
}

impl Plotter for PngPlotter {
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

use png::{BitDepth, ColorType, Encoder};
use std::mem;
use std::slice;
use surfman::{Connection, ContextAttributeFlags, ContextAttributes, GLApi, GLVersion as SurfmanGLVersion};
use surfman::{SurfaceAccess, SurfaceType};

fn render(scene: &mut Scene, output: PathBuf) {

    let view_box = dbg!(scene.view_box());
    let size = view_box.size().ceil().to_i32();
    let transform = Transform2F::from_translation(-view_box.origin());

    let connection = Connection::new().unwrap();
    //let native_widget = connection.create_native_widget_from_winit_window(&window).unwrap();
    let adapter = connection.create_adapter().unwrap();
    let mut device = connection.create_device(&adapter).unwrap();

    // Request an OpenGL 3.x context. Pathfinder requires this.
    let context_attributes = ContextAttributes {
        version: SurfmanGLVersion::new(3, 0),
        flags: ContextAttributeFlags::ALPHA,
    };
    let context_descriptor = device.create_context_descriptor(&context_attributes).unwrap();

    // Make the OpenGL context via `surfman`, and load OpenGL functions.
    let surface_type = SurfaceType::Generic { size: Size2D::new(size.x(), size.y()) };
    let mut context = device.create_context(&context_descriptor, None).unwrap();
    let surface = device.create_surface(&context, SurfaceAccess::GPUOnly, surface_type)
                        .unwrap();
    device.bind_surface_to_context(&mut context, surface).unwrap();
    device.make_context_current(&context).unwrap();
    gl::load_with(|symbol_name| device.get_proc_address(&context, symbol_name));

    let framebuffer_size = vec2i(size.x() as i32, size.y() as i32);

    // Create a Pathfinder GL device.
    let default_framebuffer = device.context_surface_info(&context)
                                    .unwrap()
                                    .unwrap()
                                    .framebuffer_object;
    let pathfinder_device = GLDevice::new(GLVersion::GL3, default_framebuffer);

    // Create a Pathfinder renderer.
    let mode = RendererMode::default_for_device(&pathfinder_device);
    let options = RendererOptions {
        dest: DestFramebuffer::full_window(framebuffer_size),
        background_color: Some(ColorF::white()),
        ..RendererOptions::default()
    };
    let resource_loader = EmbeddedResourceLoader::new();
    let mut renderer = Renderer::new(pathfinder_device, &resource_loader, mode, options);

    scene.build_and_render(&mut renderer, BuildOptions::default(), RayonExecutor);
    let mut pixels: Vec<u8> = vec![0; size.x() as usize * size.y() as usize * 4];

    unsafe {
        gl::ReadPixels(
            0,
            0,
            size.x(),
            size.y(),
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            pixels.as_mut_ptr() as *mut GLvoid,
        );
    }

    let file = File::create(output).unwrap();
    let mut encoder = Encoder::new(
        file,
        size.x() as u32,
        size.y() as u32,
    );
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut image_writer = encoder.write_header().unwrap();
    image_writer.write_image_data(&pixels).unwrap();

    // Clean up.
    drop(device.destroy_context(&mut context));
}

