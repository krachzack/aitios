
pub struct SubstanceMap {
    width: usize,
    height: usize,
    substance_idx: usize,
    entity_idx: usize,
    concentrations: Vec<f32>
}

impl SubstanceMap {
    pub fn new(width: usize, height: usize, substance_idx: usize, entity_idx: usize, concentrations: Vec<f32>) -> SubstanceMap {
        SubstanceMap {
            width, height, substance_idx, entity_idx, concentrations
        }
    }

    pub fn width(&self) -> usize { self.width }

    pub fn height(&self) -> usize { self.height }

    pub fn substance_idx(&self) -> usize { self.substance_idx }

    #[allow(unused)]
    pub fn entity_idx(&self) -> usize { self.entity_idx }

    pub fn sample_for_image_coords(&self, image_x: usize, image_y: usize, image_width: usize, image_height: usize) -> f32 {
        assert!(
            image_x < image_width && image_y < image_height,
            "Coordinates ({}/{}) out of bounds for image of size ({}/{})", image_x, image_y, image_width, image_height
        );

        // Offset by 0.5 so the center of the pixel will be sampled
        let u = (0.5 + image_x as f32) / (image_width as f32);
        // Image pixels are y-down, inverse the y axis
        let v = (0.5 + (image_height - image_y) as f32) / (image_height as f32);

        self.sample(u, v)
    }

    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let (x, y) = self.sample_xy_clamp(u, v);
        let offset = y * self.width + x;
        self.concentrations[offset]
    }

    fn sample_xy_clamp(&self, u: f32, v: f32) -> (usize, usize) {
        (
            match u {
                u if u >= 1.0 => self.width - 1,
                u if u < 0.0 => 0,
                u => (u * (self.width as f32)) as usize
            },
            match v {
                v if v >= 1.0 => self.height - 1,
                v if v < 0.0 => 0,
                v => (v * (self.height as f32)) as usize
            }
        )
    }
}
