use std::fs::File;
use std::path::Path;
use ::geom::surf::Surface;
use ::geom::scene::Scene;
use ::image;
use image::GenericImage;
use image::Pixel;

use ::kdtree::kdtree::{Kdtree, KdtreePointTrait};

//use ::cgmath::Vector2;

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

#[derive(PartialEq, Copy, Clone)]
struct SurfelTexelIndex {
    texcoords: [f64; 2],
    surfel_idx: Option<usize>
}

impl KdtreePointTrait for SurfelTexelIndex {
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
            ((y as f64) + 0.5) / (bin_count_y as f64)
        ));

    let nearest_surfel_indexes = texel_center_uvs.map(|(u, v)| texcoord_tree.nearest_search(&SurfelTexelIndex {
        texcoords: [u, v],
        surfel_idx: None
    }).surfel_idx.unwrap());

    nearest_surfel_indexes.map(|idx| surface.samples[idx].substances[substance_idx])
        .collect()
}

pub struct DensityMap {
    texture_width: usize,
    texture_height: usize
}

impl DensityMap {
    pub fn new(texture_width: usize, texture_height: usize) -> DensityMap {
        DensityMap { texture_width, texture_height }
    }
}

impl Effect for DensityMap {
    fn perform(&self, scene: &Scene, surface: &Surface) {
        let substance_count = surface.samples[0].substances.len();

        let tex_width = self.texture_width as u32;
        let tex_height = self.texture_height as u32;

        println!("Collecting density maps in resolution {}x{} for {} substances...", tex_width, tex_height, substance_count);

        let texes = scene.entities.iter().enumerate()
            .flat_map(|(entity_idx, e)| {
                let texcoord_tree = build_surfel_texel_tree(surface, entity_idx);

                (0..substance_count).map(move |substance_idx| {
                    let filename = format!("testdata/{}-{}-substance-{}-{}x{}.png", entity_idx, e.name, substance_idx, tex_width, tex_height);
                    let tex_buf = image::ImageBuffer::from_fn(
                        tex_width, tex_height,
                        // Initialize with magenta so we see the texels that do not have a surfel nearby
                        |x, y| {
                            let u = ((x as f64) + 0.5) / (tex_width as f64);
                            let v = ((y as f64) + 0.5) / (tex_height as f64);

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

        for (filename, tex) in texes {
            println!("Writing {}...", filename);
            let fout = &mut File::create(filename).unwrap();
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


/*
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
        println!("WARNING: Degenerate surfel UVs: [{}, {}], clamping to 1.0", texcoords.x, 1.0 - texcoords.y);

        if x >= texel_count_x { x = texel_count_x - 1; }
        if y >= texel_count_y { y = texel_count_y - 1; }
    }

    y * texel_count_x + x
}
*/
