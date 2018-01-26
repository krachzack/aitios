use super::substance_map::SubstanceMap;
use super::substance_map_material::SubstanceMapMaterialEffect;

use ::geom::scene::Entity;

use ::tobj::Material;

use ::image::{self, DynamicImage, GenericImage, ImageBuffer, Pixel, Rgba};

use std::collections::HashMap;

use std::fs::File;

use std::path::{PathBuf, Path};

pub struct Ramp {
    target_material_names: Vec<String>,
    texture_base_path: PathBuf,
    target_texture: TextureKind,
    segments: Vec<RampSegment>,
}

enum TextureKind {
    Diffuse, // kd in MTL
    #[allow(unused)]
    Specular // ks in MTL, metallicity
}

pub struct RampSegment {
    /// Minimum substance concentration (inclusive) for this segment to apply
    min: f32,
    /// Maximum substance concentration (exclusive) for this segment to apply
    max: f32,
    // If none, uses the original material image
    min_texture: Option<DynamicImage>,
    max_texture: Option<DynamicImage>
}

impl Ramp {
    pub fn new(target_material_names: Vec<String>, texture_base_path: PathBuf, segments: Vec<RampSegment>) -> Ramp {
        Ramp { target_material_names, texture_base_path, target_texture: TextureKind::Diffuse, segments }
    }

    fn should_process_material(&self, original_material: &Material) -> bool {
        self.target_material_names.iter().any(|n| n == &original_material.name)
    }
}

impl SubstanceMapMaterialEffect for Ramp {

    fn perform(&self, _entity: &Entity, original_material: &Material, concentrations: &SubstanceMap, output_file_prefix: &Path) -> Option<Material> {

        if !self.should_process_material(original_material) {
            return None;
        }

        let width = concentrations.width() as u32;
        let height = concentrations.height() as u32;

        let original_texture = original_texture(&self.texture_base_path, original_material, &self.target_texture);

        let fallback_color = Rgba { data: [0, 0, 255, 255] };

        let synthesized_texture = ImageBuffer::from_fn(
            width, height,
            |x, y| {
                let concentration = {
                    let val = concentrations.sample_for_image_coords(x as usize, y as usize, width as usize, height as usize);
                    if val >= 1.0 { 0.9999999999 } else { val }
                };

                if !concentration.is_finite() {
                    fallback_color
                } else {
                    let seg = self.segments.iter()
                        .find(|s| s.in_range(concentration));

                    if let Some(seg) = seg {
                        let u = (x as f32) / (width as f32);
                        let v = (y as f32) / (height as f32);

                        seg.interpolate(u, v, concentration, &original_texture).unwrap()
                    } else {
                        fallback_color
                    }
                }
            }
        );

        let material_name = format!("{}-ramp-map-substance-{}", output_file_prefix.file_name().unwrap().to_str().unwrap(), concentrations.substance_idx());

        let mut ramp_map_path = PathBuf::from(output_file_prefix);
        ramp_map_path.set_file_name(&material_name);
        ramp_map_path.set_extension("png");

        let target_filename_relative = String::from(ramp_map_path.file_name().unwrap().to_str().unwrap());

        info!("Writing ramp texture {}...", target_filename_relative);
        let ramp_map_file = &mut File::create(ramp_map_path).unwrap();

        image::ImageRgba8(synthesized_texture).save(ramp_map_file, image::PNG)
                .expect("Substance map texture could not be written");

        Some(material_with_diffuse(&material_name, &target_filename_relative))
    }
}

impl RampSegment {
    pub fn new(min: f32, max: f32, min_texture: Option<DynamicImage>, max_texture: Option<DynamicImage>) -> RampSegment {
        RampSegment { min, max, min_texture, max_texture }
    }

    fn in_range(&self, concentration: f32) -> bool {
        self.min <= concentration && concentration < self.max
    }

    fn interpolate(&self, u: f32, v: f32, concentration: f32, original_texture: &DynamicImage) -> Option<Rgba<u8>> {
        if let Some(alpha) = self.alpha(concentration) {
            let min_texture = self.min_texture.as_ref().unwrap_or(original_texture);
            let max_texture = self.max_texture.as_ref().unwrap_or(original_texture);

            let min_px = sample(min_texture, u, v);
            let max_px = sample(max_texture, u, v);
            let middle_px = min_px.map2(
                &max_px,
                |src, target| ((1.0 - alpha) * (src as f32) + alpha * (target as f32)) as u8
            );

            Some(middle_px)
        } else {
            None
        }
    }

    fn alpha(&self, concentration: f32) -> Option<f32> {
        let alpha = (concentration - self.min) / (self.max - self.min);

        // TODO This is linear, maybe cubic interpolation should be possible too?

        // min inclusive, max exclusive
        if alpha >= 0.0 && alpha < 1.0 {
            Some(alpha)
        } else {
            None
        }
    }
}

fn sample<T : GenericImage>(texture: &T, u: f32, v: f32) -> T::Pixel {
    let (width, height) = texture.dimensions();
    texture.get_pixel((u * (width as f32)) as u32, (v * (height as f32)) as u32)
}

fn original_texture(texture_base_path: &PathBuf, material: &Material, kind: &TextureKind) -> DynamicImage {
    let mut texture_path = texture_base_path.clone();

    match kind {
        &TextureKind::Diffuse => texture_path.push(&material.diffuse_texture),
        &TextureKind::Specular => texture_path.push(&material.specular_texture)
    }

    image::open(&texture_path)
                .expect("Texture of material could not be loaded for weathering")
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
