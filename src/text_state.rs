use pdf::content::Matrix;

#[derive(Clone, Debug)]
pub struct TextState {
    //pub font: Font,
    pub font_size: f32,
    pub font_matrix: Matrix,
    pub text_matrix: Matrix,
    pub text_line_matrix: Matrix,
}

impl TextState {
    pub fn new() -> Self {
        Self {
            //font: Font::
            font_size: 0.0,
            font_matrix: Matrix::default(),
            text_matrix: Matrix::default(),
            text_line_matrix: Matrix::default(),
        }
    }
}
