use std::fs::File;
use std::path::Path;
use ::geom::surf::Surface;
use ::geom::scene::Scene;
use ::image;
use image::GenericImage;
use image::Pixel;

pub trait Effect {
    fn perform(&self, scene: &Scene, surface: &Surface);
}

pub struct Blend {
    /// Index of substance that drives the blend
    /// Density of 0 equals original texture, 1 equals the texture to blend to
    substance_idx: usize,
    /// Name of the material that this blend effect should affect
    subject_material_name: String,
    /// Name of the map inside the material that should be changed specifically,
    /// Follows OBJ conventions, e.g. map_Kd is the diffuse map.
    subject_material_map: String,
    /// Material that the subject map should be blended towards
    blend_towards_tex_file: String
}

impl Effect for Blend {
    fn perform(&self, scene: &Scene, surface: &Surface) {
        let subject_map = self.load_subject_map(scene);
        let towards_map = self.load_towards_map();

        assert_eq!(subject_map.dimensions(), towards_map.dimensions());
        let (tex_width, tex_height) = subject_map.dimensions();

        let subject_material_idx = scene.materials.iter().position(|m| m.name == self.subject_material_name).unwrap();

        for (entity_idx, entity) in scene.entities.iter().enumerate().filter(|&(_, e)| e.material_idx == subject_material_idx) {
            let blend_factors = blend_factors_by_avg_local_density(surface, self.substance_idx, entity_idx, tex_width as usize, tex_height as usize);

            let blended_map = image::ImageBuffer::from_fn(
                subject_map.width(), subject_map.height(),
                |x, y| {
                    let factor = blend_factors[(y*tex_width + x) as usize];
                    subject_map.get_pixel(x, y).map2(
                        &towards_map.get_pixel(x, y),
                        |c0, c1| ((1.0 - factor) * (c0 as f32) + factor * (c1 as f32)) as u8
                    )
                }
            );

            let target_filename = format!("testdata/{}-{}-{}-{}-weathered.png", entity_idx, entity.name, self.subject_material_name, self.subject_material_map);
            println!("Writing effect texture {}...", target_filename);
            let fout = &mut File::create(target_filename).unwrap();

            image::ImageRgba8(blended_map).save(fout, image::PNG).unwrap();
        }
    }
}

impl Blend {
    pub fn new(substance_idx: usize, subject_material_name: &str, subject_material_map: &str, blend_towards_tex_file: &str) -> Blend {
        Blend {
            substance_idx,
            subject_material_name: String::from(subject_material_name),
            subject_material_map: String::from(subject_material_map),
            blend_towards_tex_file: String::from(blend_towards_tex_file)
        }
    }

    fn load_subject_map(&self, scene: &Scene) -> image::DynamicImage {
        let texture_base_path = "testdata/";

        // Panics if subject material is not there, this is a little late for that check
        let subject_material_idx = scene.materials.iter().position(|m| m.name == self.subject_material_name).unwrap();
        let subject_material = &scene.materials[subject_material_idx];
        let subject_map = match self.subject_material_map.as_ref() {
            "map_Kd" => &subject_material.diffuse_texture,
            _ => panic!(format!("Unknown subject map {}, try map_Kd", self.subject_material_map))
        };
        let subject_map = format!("{}{}", texture_base_path, subject_map);
        // Panics if texture cannot be loaded
        let subject_map = image::open(&Path::new(&subject_map)).unwrap();

        subject_map
    }

    fn load_towards_map(&self) -> image::DynamicImage {
        let texture_base_path = "testdata/";
        let towards_map = format!("{}{}", texture_base_path, self.blend_towards_tex_file);

        image::open(&Path::new(&towards_map)).unwrap()
    }
}

// TODO maybe use sum, not avg?
fn blend_factors_by_avg_local_density(surface: &Surface, substance_idx: usize, entity_idx: usize, tex_width: usize, tex_height: usize) -> Vec<f32> {
    substance_density_bins(surface, substance_idx, entity_idx, tex_width, tex_height)
        .iter()
        .map(|b| {
            if b.is_empty() {
                0.0
            } else {
                // local avg
                let avg = b.iter().sum::<f32>() / (b.len() as f32);
                if avg < 0.0 { 0.0 }
                else if avg > 1.0 { 1.0 }
                else { avg }
            }
        })
        .collect()
}

/// Collects all surfels in proximity of a texel into an according bin of surfels
fn substance_density_bins(surface: &Surface, substance_idx: usize, entity_idx: usize, bin_count_x: usize, bin_count_y: usize) -> Vec<Vec<f32>> {
    let mut sample_bins = vec![Vec::new(); bin_count_x * bin_count_y];

    // FIXME filter the samples to only use the ones that affect the right material
    for sample in &surface.samples {
        if sample.entity_idx == entity_idx {
            // This cuts of the fractional part, kinda like nearest filtering
            let x = (sample.texcoords.x * (bin_count_x as f32)) as usize;
            // NOTE we use y facing down for serializing textures, but the v coordinate is typically facing up
            let y = ((1.0 - sample.texcoords.y) * (bin_count_y as f32)) as usize;

            if x >= bin_count_x || y >= bin_count_y {
                // Interpolation of texture coordinates can lead to degenerate uv coordinates
                // e.g. < 0 or > 1
                // In such cases, do not try to save the surfel but ingore it
                println!("WARNING: Degenerate surfel UVs: [{}, {}]", sample.texcoords.x, 1.0 - sample.texcoords.y);
                continue;
            }

            sample_bins[y*bin_count_x + x].push(sample.substances[substance_idx]);
        }
    }

    sample_bins
}