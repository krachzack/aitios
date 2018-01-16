
use super::substance_map::SubstanceMap;
use super::substance_map_material::SubstanceMapMaterialEffect;

use ::geom::scene::Entity;

use ::cgmath::Vector4;
use ::cgmath::prelude::*;

use ::tobj::Material;

use ::image;

use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::fs::File;

pub struct SubstanceColorEffect {
    zero_color: Vector4<f32>,
    one_color: Vector4<f32>,
    unused_color: Vector4<f32>
}

impl SubstanceColorEffect {
    pub fn new(zero_color: Vector4<f32>, one_color: Vector4<f32>, unused_color: Vector4<f32>) -> SubstanceColorEffect {
        SubstanceColorEffect {
            zero_color, one_color, unused_color
        }
    }

    fn concentration_to_color(&self, concentration: f32) -> Vector4<f32> {
        if concentration.is_finite() {
            let alpha = if concentration <= 0.0 {
                0.0
            } else if concentration >= 1.0 {
                1.0
            } else {
                concentration
            };

            self.zero_color.lerp(self.one_color, alpha)
        } else {
            self.unused_color
        }
    }
}

impl SubstanceMapMaterialEffect for SubstanceColorEffect {
    fn perform(&self, _entity: &Entity, concentrations: &SubstanceMap, output_file_prefix: &Path) -> Option<Material> {
        let width = concentrations.width();
        let height = concentrations.height();

        let substance_map = image::ImageBuffer::from_fn(
            width as u32, height as u32,
            |x, y| {
                let concentration = concentrations.sample_for_image_coords(x as usize, y as usize, width, height);

                let color = self.concentration_to_color(concentration);

                let color = [
                    (255.99999 * color.x) as u8,
                    (255.99999 * color.y) as u8,
                    (255.99999 * color.z) as u8,
                    (255.99999 * color.w) as u8
                ];

                image::Rgba { data: color }
            }
        );

        let material_name = format!("{}-density-map-substance-{}", output_file_prefix.file_name().unwrap().to_str().unwrap(), concentrations.substance_idx());

        let mut substance_map_path = PathBuf::from(output_file_prefix);
        substance_map_path.set_file_name(&material_name);
        substance_map_path.set_extension("png");

        let target_filename_relative = String::from(substance_map_path.file_name().unwrap().to_str().unwrap());

        info!("Writing substance map texture {}...", target_filename_relative);
        let substance_map_file = &mut File::create(substance_map_path).unwrap();

        image::ImageRgba8(substance_map).save(substance_map_file, image::PNG)
                .expect("Substance map texture could not be written");

        Some(material_with_diffuse(&material_name, &target_filename_relative))
    }
}

fn material_with_diffuse(name: &str, diffuse: &str) -> Material {
    Material {
        name: String::from(name),
        ambient: [1.0; 3],
        diffuse: [1.0; 3],
        specular: [0.0; 3],
        shininess: 0.0,
        dissolve: 1.0,
        optical_density: 1.0,
        ambient_texture: String::new(),
        diffuse_texture: String::from(diffuse),
        specular_texture: String::new(),
        normal_texture: String::new(),
        dissolve_texture: String::new(),
        illumination_model: Some(0), // color on and ambient off
        unknown_param: HashMap::new()
    }
}
