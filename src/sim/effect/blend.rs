use super::substance_map::SubstanceMap;
use super::substance_map_material::SubstanceMapMaterialEffect;

use ::geom::scene::Entity;

use ::tobj::Material;

use ::image::{self, DynamicImage, GenericImage, ImageBuffer, Pixel};

use std::collections::HashMap;

use std::fs::File;

use std::path::{PathBuf, Path};

pub struct Blend {
    target_material_names: Vec<String>,
    texture_base_path: PathBuf,
    overlay_image: DynamicImage
}

impl Blend {
    pub fn new<P : Into<PathBuf>>(target_material_names: Vec<String>, texture_base_path: P, overlay_image_path: &Path) -> Blend {
        let texture_base_path = texture_base_path.into();

        let overlay_image = image::open(overlay_image_path)
            .expect(&format!("Blend target image {:?} could not be loaded", overlay_image_path));

        Blend {
            target_material_names,
            texture_base_path,
            overlay_image
        }
    }

    fn should_process_material(&self, original_material: &Material) -> bool {
        self.target_material_names.iter().any(|n| n == &original_material.name)
    }
}

fn blend_by_substance_map(original: &DynamicImage, overlay: &DynamicImage, concentrations: &SubstanceMap) -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let width = concentrations.width() as u32;
    let height = concentrations.height() as u32;

    ImageBuffer::from_fn(
        width, height,
        |x, y| {
            let u = (x as f32) / (width as f32);
            let v = (y as f32) / (height as f32);

            let (original_w, original_h) = original.dimensions();
            let original_x = (u * (original_w as f32)) as u32;
            let original_y = (v * (original_h as f32)) as u32;
            let original_px = original.get_pixel(original_x, original_y);

            let (overlay_w, overlay_h) = overlay.dimensions();
            let overlay_x = (u * (overlay_w as f32)) as u32;
            let overlay_y = (v * (overlay_h as f32)) as u32;
            let overlay_px = overlay.get_pixel(overlay_x, overlay_y);

            let concentration = concentrations.sample_for_image_coords(x as usize, y as usize, width as usize, height as usize);

            original_px.map2(
                &overlay_px,
                |src, target| ((1.0 - concentration) * (src as f32) + concentration * (target as f32)) as u8
            )
        }
    )
}

impl SubstanceMapMaterialEffect for Blend {

    fn perform(&self, _entity: &Entity, original_material: &Material, concentrations: &SubstanceMap, output_file_prefix: &Path) -> Option<Material> {

        if !self.should_process_material(original_material) {
            return None;
        }

        let original_material_diffuse_tex = {
            let mut diffuse_texture_path = self.texture_base_path.clone();
            diffuse_texture_path.push(&original_material.diffuse_texture);
            image::open(&diffuse_texture_path)
                .expect("Diffuse texture of material could not be loaded for weathering")
        };

        let blent = blend_by_substance_map(&original_material_diffuse_tex, &self.overlay_image, concentrations);

        let material_name = format!("{}-blent-map-substance-{}", output_file_prefix.file_name().unwrap().to_str().unwrap(), concentrations.substance_idx());

        let mut blent_map_path = PathBuf::from(output_file_prefix);
        blent_map_path.set_file_name(&material_name);
        blent_map_path.set_extension("png");

        let target_filename_relative = String::from(blent_map_path.file_name().unwrap().to_str().unwrap());

        info!("Writing blent texture {}...", target_filename_relative);
        let blent_map_file = &mut File::create(blent_map_path).unwrap();

        image::ImageRgba8(blent).save(blent_map_file, image::PNG)
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
