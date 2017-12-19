use std::fs::File;
use std::path::{Path, PathBuf};
use ::geom::surf::Surface;
use ::geom::scene::Scene;
use ::image;
use image::GenericImage;
use image::Pixel;

use ::kdtree::kdtree::{Kdtree, KdtreePointTrait};

use ::tobj::Material;

use ::cgmath::Vector2;

/// A trait for effects that mutate the scene using the information in an associated
/// surface model.
pub trait Effect {
    fn perform(&self, scene: &mut Scene, surface: &Surface);
}

/// An effect for blending a specific texture map, e.g. the diffuse color map
/// towards another texture file, according to substance concentration in the
/// surfels associated with each texel.
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
    blend_towards_tex_file: String,
    material_directory: PathBuf,
    output_directory: PathBuf
}

impl Effect for Blend {
    fn perform(&self, scene: &mut Scene, surface: &Surface) {
        info!("Blending textures for material {}", self.subject_material_name);

        let subject_map = self.load_subject_map(scene);
        let towards_map = self.load_towards_map();

        assert_eq!(subject_map.dimensions(), towards_map.dimensions());
        let (tex_width, tex_height) = subject_map.dimensions();

        let subject_material_idx = scene.materials.iter()
            .position(|m| m.name == self.subject_material_name)
            .expect(&format!("No material with specified name {}", self.subject_material_name));

        let entities = &mut scene.entities;
        let materials = &mut scene.materials;

        for (entity_idx, entity) in entities.iter_mut().enumerate().filter(|args| args.1.material_idx == subject_material_idx) {
            let blend_factors = blend_factors_by_closest_surfel(surface, self.substance_idx, entity_idx, tex_width as usize, tex_height as usize);

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

            let mut target_filename = self.output_directory.clone();
            target_filename.push(format!("{}-{}-{}-{}-weathered", entity_idx, entity.name, self.subject_material_name, self.subject_material_map));
            target_filename.set_extension("png");

            let target_filename_relative = String::from(target_filename.file_name().unwrap().to_str().unwrap());

            info!("Writing effect texture {:?}...", target_filename);
            let fout = &mut File::create(target_filename).unwrap();

            image::ImageRgba8(blended_map).save(fout, image::PNG)
                .expect("Blended map could not be written");

            let blended_material_idx = duplicate_material(materials, subject_material_idx);
            let blended_material = &mut materials[blended_material_idx];
            blended_material.name = format!("{}-entity-{}-{}-substance-{}-blended", blended_material.name, entity_idx, entity.name, self.substance_idx);

            match self.subject_material_map.as_ref() {
                "map_Kd" => blended_material.diffuse_texture = target_filename_relative,
                _ => panic!("Unknown subject map {}, try map_Kd", self.subject_material_map)
            }

            entity.material_idx = blended_material_idx;
        }
    }
}

fn duplicate_material(materials: &mut Vec<Material>, original_idx: usize) -> usize {
    let material = materials[original_idx].clone();

    let new_material_idx = materials.len();
    materials.push(material);

    new_material_idx
}

impl Blend {
    pub fn new(substance_idx: usize, material_directory: &Path, subject_material_name: &str, subject_material_map: &str, blend_towards_tex_file: &str, output_directory: &Path) -> Blend {
        Blend {
            substance_idx,
            subject_material_name: String::from(subject_material_name),
            subject_material_map: String::from(subject_material_map),
            blend_towards_tex_file: String::from(blend_towards_tex_file),
            material_directory: PathBuf::from(material_directory),
            output_directory: PathBuf::from(output_directory)
        }
    }

    fn load_subject_map(&self, scene: &Scene) -> image::DynamicImage {

        let subject_material = {
            // Panics if subject material is not there, this is a little late for that check
            let subject_material_idx = scene.materials.iter()
                .position(|m| m.name == self.subject_material_name)
                .expect(&format!("Unknown material {} for blend effect", self.subject_material_name));

            &scene.materials[subject_material_idx]
        };

        let subject_map_path = {
            let subject_map_filename = match self.subject_material_map.as_ref() {
                "map_Kd" => &subject_material.diffuse_texture,
                _ => panic!("Unknown subject map {}, try map_Kd", self.subject_material_map)
            };

            let mut subject_map_path = self.material_directory.clone();
            subject_map_path.push(subject_map_filename);
            subject_map_path
        };

        // Panics if texture cannot be loaded
        let subject_map = image::open(&subject_map_path)
            .expect(&format!("Subject map at {:?} not found", subject_map_path));

        subject_map
    }

    fn load_towards_map(&self) -> image::DynamicImage {
        image::open(&Path::new(&self.blend_towards_tex_file))
            .expect(&format!("Blend towards map at {:?} not found", self.blend_towards_tex_file))
    }
}

#[derive(PartialEq, Copy, Clone)]
struct SurfelTexelIndex {
    texcoords: [f64; 2],
    surfel_idx: Option<usize>
}

impl KdtreePointTrait for SurfelTexelIndex {
    #[inline]
    fn dims(&self) -> &[f64] {
        &self.texcoords
    }
}

fn blend_factors_by_closest_surfel(surface: &Surface, substance_idx: usize, entity_idx: usize, bin_count_x: usize, bin_count_y: usize) -> Vec<f32> {
    let texcoord_tree = build_surfel_texel_tree(surface, entity_idx);

    // (0,0), (1,0), (2,0), [...], (bin_count_x-1, bin_count_y-1)
    let texel_integer_coords = (0..bin_count_y).flat_map(|y| (0..bin_count_x).map(move |x| (x,y)));

    // As UVs in the middle of the pixel, hence +0.5
    let texel_center_uvs = texel_integer_coords
        .map(|(x, y)| (
            ((x as f64) + 0.5) / (bin_count_x as f64),
            (((bin_count_y - y) as f64) + 0.5) / (bin_count_y as f64)
        ));

    let nearest_surfel_indexes = texel_center_uvs.map(|(u, v)| texcoord_tree.nearest_search(&SurfelTexelIndex {
        texcoords: [u, v],
        surfel_idx: None
    }).surfel_idx.unwrap());

    nearest_surfel_indexes.map(|idx| surface.samples[idx].substances[substance_idx])
        // 1:1 mapping of density to blend factor, but clamp values over 1.0
        .map(|density| if density > 1.0 { 1.0 } else { 0.0 })
        .collect()
}

/// An effect for writing density maps in a specific resolution to a preconfigured
/// output directory. The effect will not mutate the scene and only create the density
/// maps without associating them with the scene. The maps are for debugging purposes
/// and can be set manually as the material.
pub struct DensityMap {
    texture_width: usize,
    texture_height: usize,
    output_directory: PathBuf,
}

impl DensityMap {
    pub fn new(texture_width: usize, texture_height: usize, output_directory: &str) -> DensityMap {
        DensityMap { texture_width, texture_height, output_directory: PathBuf::from(output_directory) }
    }
}

impl Effect for DensityMap {
    fn perform(&self, scene: &mut Scene, surface: &Surface) {
        info!("Writing density maps to {:?}", self.output_directory);

        let substance_count = surface.samples[0].substances.len();

        let tex_width = self.texture_width as u32;
        let tex_height = self.texture_height as u32;

        info!("Collecting density maps in resolution {}x{} for {} substances...", tex_width, tex_height, substance_count);

        let texes = scene.entities.iter().enumerate()
            .flat_map(|(entity_idx, e)| {
                let texcoord_tree = build_surfel_texel_tree(surface, entity_idx);

                (0..substance_count).map(move |substance_idx| {
                    let mut filename = self.output_directory.clone();
                    filename.push(format!("{}-{}-substance-{}-{}x{}", entity_idx, e.name, substance_idx, tex_width, tex_height));
                    filename.set_extension("png");

                    let tex_buf = image::ImageBuffer::from_fn(
                        tex_width, tex_height,
                        |x, y| {
                            let u = ((x as f64) + 0.5) / (tex_width as f64);
                            // pixels are y down, reverse v coordinate when calculating uv coordinates
                            let v = (((tex_height - y) as f64) + 0.5) / (tex_height as f64);

                            let surfel_idx = texcoord_tree.nearest_search(&SurfelTexelIndex {
                                texcoords: [u, v],
                                surfel_idx: None
                            }).surfel_idx.unwrap();

                            let substance_density = surface.samples[surfel_idx].substances[substance_idx];
                            let luminosity = if substance_density > 1.0 { 255u8 } else { (substance_density * 255.0) as u8 };

                            image::Rgb([luminosity, luminosity, luminosity])
                        }
                    );

                    (filename, tex_buf)
                })
            });

        for (path, tex) in texes {
            info!("Writing {:?}...", path);
            let fout = &mut File::create(path).unwrap();
            image::ImageRgb8(tex).save(fout, image::PNG).unwrap();
        }
    }
}

fn build_surfel_texel_tree(surface: &Surface, entity_idx: usize) -> Kdtree<SurfelTexelIndex> {
    let mut indexes : Vec<_> = surface.samples.iter()
        .enumerate()
        .filter(|&(_, s)| s.entity_idx == entity_idx)
        .map(|(surfel_idx, s)| {
            let u = s.texcoords.x as f64;
            let v = s.texcoords.y as f64;

            SurfelTexelIndex {
                texcoords: [u, v],
                surfel_idx: Some(surfel_idx)
            }
        })
        .collect();

    Kdtree::new(&mut indexes)
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
            let texel_idx = nearest_texel_idx_clamp(sample.texcoords, bin_count_x, bin_count_y);

            sample_bins[texel_idx].push(sample.substances[substance_idx]);
        }
    }

    sample_bins
}

/// This finds the closest pixel to the uvs, kinda like nearest filtering with clamp
fn nearest_texel_idx_clamp(texcoords: Vector2<f32>, texel_count_x: usize, texel_count_y: usize) -> usize {
    // This cuts of the fractional part, kinda like nearest filtering
    let mut x = (texcoords.x * (texel_count_x as f32)) as usize;
    // NOTE we use y facing down for serializing textures, but the v coordinate is typically facing up
    let mut y = ((1.0 - texcoords.y) * (texel_count_y as f32)) as usize;

    if x >= texel_count_x || y >= texel_count_y {
        // Interpolation of texture coordinates can lead to degenerate uv coordinates
        // e.g. < 0 or > 1
        // In such cases, do not try to save the surfel but ingore it
        warn!("WARNING: Degenerate surfel UVs: [{}, {}], clamping to 1.0", texcoords.x, 1.0 - texcoords.y);

        if x >= texel_count_x { x = texel_count_x - 1; }
        if y >= texel_count_y { y = texel_count_y - 1; }
    }

    y * texel_count_x + x
}

