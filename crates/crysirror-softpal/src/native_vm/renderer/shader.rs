#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShaderProgram {
    DrawPrimitiveUp2d,
    Bicubic,
    RadialBlur,
    Raster,
    RasterSw,
}

impl ShaderProgram {
    pub fn label(self) -> &'static str {
        match self {
            Self::DrawPrimitiveUp2d => "PAL FX DRAW_PRIMITIVE_UP2D",
            Self::Bicubic => "PAL FX BICUBIC",
            Self::RadialBlur => "PAL FX RADIAL_BLUR",
            Self::Raster => "PAL FX RASTER",
            Self::RasterSw => "PAL FX RASTER_SW",
        }
    }
}

pub fn shader_source(program: ShaderProgram) -> &'static str {
    match program {
        ShaderProgram::DrawPrimitiveUp2d => include_str!("shaders/draw_primitive_up_2d.wgsl"),
        ShaderProgram::Bicubic => include_str!("shaders/bicubic.wgsl"),
        ShaderProgram::RadialBlur => include_str!("shaders/radial_blur.wgsl"),
        ShaderProgram::Raster => include_str!("shaders/raster.wgsl"),
        ShaderProgram::RasterSw => include_str!("shaders/raster_sw.wgsl"),
    }
}
