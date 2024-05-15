use std::path::PathBuf;

extern crate pathfinder_geometry as g;

//mod common;
mod plotter;
mod graphics_state;
mod text_state;
mod render;
//mod screen_plotter;
mod vector_plotter;
mod png;

use clap::Parser;
use g::rect::RectF;
use g::transform2d::Transform2F;
use g::vector::Vector2F;
use pdf::file::FileOptions;
use pdf::object::{Page, Rect};
use pdf::PdfError;

use crate::render::RenderState;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file
    #[arg(short, long)]
    input: PathBuf,

    /// Page number
    #[arg(short, long, default_value_t = 0)]
    page: u32,

    /// Output file
    #[arg(short, long)]
    output: PathBuf,
}

//const SCALE: f32 = 25.4 / 72.;
const SCALE: f32 = 1.0;

pub fn page_bounds(page: &Page) -> g::rect::RectF {
    let Rect { left, right, top, bottom } = page.media_box().expect("no media box");
    g::rect::RectF::from_points(g::vector::Vector2F::new(left, bottom), g::vector::Vector2F::new(right, top)) * SCALE
}

fn main() -> Result<(), PdfError>{
    let args = Args::parse();
    convert(args.input, args.output, args.page)
}

pub fn convert(input: PathBuf, output: PathBuf, page_nr: u32) -> Result<(), PdfError>{

    let file = FileOptions::cached().open(input).unwrap();
    let mut resolve = file.resolver();
    let page = file.get_page(page_nr).expect("no such page");

        let transform = Transform2F::default();

        let bounds = page_bounds(&page);
        let rotate = Transform2F::from_rotation(page.rotate as f32 * std::f32::consts::PI / 180.);
        let br = rotate * RectF::new(Vector2F::zero(), bounds.size());
        let translate = Transform2F::from_translation(Vector2F::new(
            -br.min_x().min(br.max_x()),
            -br.min_y().min(br.max_y()),
        ));
        let view_box = transform * translate * br;

        let root_transformation = transform
            * translate
            * rotate
            * Transform2F::row_major(SCALE, 0.0, -bounds.min_x(), 0.0, -SCALE, bounds.max_y());

        let resources = pdf::t!(page.resources());

    let mut plotter = vector_plotter::VectorPlotter::new(view_box);
    let mut plotter = png::PngPlotter::new(view_box);
    //let mut plotter = screen_plotter::ScreenPlotter::new(view_box);
    let mut render = RenderState::new(&mut plotter, &mut resolve, resources, root_transformation);
    render.render(&page)?;
    plotter.write(output);

    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::Path;

    //test convert sample pdf file to svg
    #[test]
    fn test_pdf_to_svg() {
        super::convert(Path::new("rack.pdf").to_path_buf(), Path::new("rack.png").to_path_buf(), 0).unwrap();
    }
}
