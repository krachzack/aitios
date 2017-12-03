
use ::geom::surf::Surface;

struct Blend {
    /// Texture that equals a texture density of 0.0
    tex_file_zero: String,
    /// Texture that equals a texture density of 1.0
    tex_file_one: String,
    /// Substance that is the source of interpolation
    substance_idx: usize,
    /// Material that should be affected
    material_idx: usize
}

impl Blend {
    pub fn perform(&self, surface: Surface) {

    }
}
