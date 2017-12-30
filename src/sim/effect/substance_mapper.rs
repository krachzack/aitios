use super::scene::SceneEffect;
use super::substance_map::SubstanceMap;
use super::substance_map_material::SubstanceMapMaterialEffect;

use ::geom::scene::Scene;
use ::geom::surf::Surface;
use ::geom::tri::Triangle;
use ::geom::vtx::Vertex;

use ::cgmath::{Vector2, Vector3};

use ::nearest_kdtree::KdTree;
use ::nearest_kdtree::distance::squared_euclidean;

use std::f32::NAN;
use std::time::Instant;

pub struct SubstanceMapper {
    substance_idx: usize,
    sampling: Sampling,
    texture_width: usize,
    texture_height: usize,
    after_effects: Vec<Box<SubstanceMapMaterialEffect>>
}

/// Sets the strategy for surfel lookup for a given texel
pub enum Sampling {
    /// Calculates a texture pixel by looking up all surfels within the
    /// given radius in UV space and taking the average
    /// The UV distance breaks down on seams but approximates geodesic distance.
    /// Also, surfels of other entities cannot affect the surfels of this entity,
    /// since they do not share a common UV space.
    #[allow(dead_code)]
    UvRadius(f32),
    /// Approximates a world-space position for each UV coordinate by looking
    /// up surfels nearby in UV space and approximating a position using their
    /// positions and UV coordinates.
    ///
    /// Then, near surfels are looked up in position space and these are collected.
    /// When the radius is larger than the sampling distance, every texel should have
    /// multiple surfels to work with
    SpaceRadius(f32)
}

impl SceneEffect for SubstanceMapper {
    fn perform_after_iteration(&self, _scene: &mut Scene, _surf: &Surface) { }

    fn perform_after_simulation(&self, scene: &mut Scene, surf: &Surface) {
        let start = Instant::now();

        for entity_idx in 0..scene.entities.len() {
            info!("Gathering {}x{} substance {} map for entity {}...", self.texture_width, self.texture_height, self.substance_idx, scene.entities[entity_idx].name);
            let substance_tex = self.gather(surf, entity_idx);
            info!("Ok, took {}s", start.elapsed().as_secs());

            for effect in &self.after_effects {
                if let Some(new_material) = effect.perform(&scene.entities[entity_idx], &substance_tex) {
                    let new_material_idx = scene.materials.len();
                    scene.materials.push(new_material);
                    scene.entities[entity_idx].material_idx = new_material_idx;
                }
            }
        }
    }
}

impl SubstanceMapper {
    pub fn new(substance_idx: usize, sampling: Sampling, texture_width: usize, texture_height: usize, after_effects: Vec<Box<SubstanceMapMaterialEffect>>) -> SubstanceMapper {
        SubstanceMapper {
            substance_idx,
            sampling,
            //sampling: Sampling::UvRadius(3.0 / (texture_width as f32)), // within three pixels distance in UV space
            texture_width,
            texture_height,
            after_effects
        }
    }

    fn gather(&self, surf: &Surface, entity_idx: usize) -> SubstanceMap {
        SubstanceMap::new(
            self.texture_width,
            self.texture_height,
            self.substance_idx,
            entity_idx,
            match self.sampling {
                Sampling::UvRadius(radius) => self.gather_uv_radius(surf, entity_idx, radius, self.texture_width, self.texture_height),
                Sampling::SpaceRadius(radius) => self.gather_space_radius(surf, entity_idx, radius, self.texture_width, self.texture_height)
            }
        )
    }

    fn gather_space_radius(&self, surf: &Surface, entity_idx: usize, radius: f32, tex_width: usize, tex_height: usize) -> Vec<f32> {
        let mut concentrations = Vec::with_capacity(tex_width * tex_height);

        let position_tree = self.build_position_uv_tree(surf, entity_idx);

        // width and height of a pixel in UV space
        let pixel_width = 1.0 / (tex_width as f64);
        let pixel_height = 1.0 / (tex_height as f64);

        let mut v = 0.5 * pixel_height;

        for _ in 0..tex_height {
            let mut u = 0.5 * pixel_width;

            for _ in 0..tex_width {
                // Position approximated by looking up the 3 nearest surfels in UV space
                // and then forming a UV-space triangle, then, the position on the given uv
                // coordinate is approximated from barycentric coordinates calculated for the
                // desired uv position within the uv triangle and using them to synthesize a position
                let uvs_and_positions : Vec<_> = position_tree.nearest(
                    &[u, v],
                    3,
                    &squared_euclidean
                ).unwrap()
                    .iter()
                    .map(|t| t.1)
                    .collect();

                let interpolated_position = Triangle::new(
                    *uvs_and_positions[0],
                    *uvs_and_positions[1],
                    *uvs_and_positions[2]
                ).interpolate_at( // Interpolate position with barys from uv space
                    Vector3::new(u as f32, v as f32, 0.0),
                    |v| v.0
                );

                if interpolated_position.x.is_finite() && interpolated_position.y.is_finite() && interpolated_position.x.is_finite() {
                    let surfels = surf.find_within_sphere(interpolated_position, radius);

                    let concentration = if surfels.is_empty() {
                        NAN
                    } else {
                        surfels.iter()
                            .map(|s| s.substances[self.substance_idx])
                            .sum::<f32>() / (surfels.len() as f32)
                    };

                    concentrations.push(concentration);
                } else {
                    warn!("Position interpolation failed for UV({}/{}), falling back to using concentration of nearest surfel", u, v);
                    concentrations.push(surf.nearest(uvs_and_positions[0].0).substances[self.substance_idx]);
                }

                u += pixel_width;
            }
            v += pixel_height;
        }

        concentrations
    }

    fn gather_uv_radius(&self, surf: &Surface, entity_idx: usize, radius: f32, tex_width: usize, tex_height: usize) -> Vec<f32> {
        let mut concentrations = Vec::with_capacity(tex_width * tex_height);

        let concentration_tree = self.build_substance_uv_tree(surf, entity_idx);

        // width and height of a pixel in UV space
        let pixel_width = 1.0 / (tex_width as f32);
        let pixel_height = 1.0 / (tex_height as f32);

        let mut v = 0.5 * pixel_height;

        for _ in 0..tex_height {
            let mut u = 0.5 * pixel_width;

            for _ in 0..tex_width {
                concentrations.push(self.gather_concentration_at(&concentration_tree, u, v, radius));
                u += pixel_width;
            }
            v += pixel_height;
        }

        concentrations
    }

    /// Builds a kdtree of substance values indexed by their position in UV space
    fn build_substance_uv_tree(&self, surf: &Surface, entity_idx: usize) -> KdTree<f32, [f64; 2]> {
        let mut tree = KdTree::new(2); //KdTree::new_with_capacity(2, surf.samples.len());

        for sample in &surf.samples {
            if sample.entity_idx == entity_idx {
                let pos = [sample.texcoords.x as f64, sample.texcoords.y as f64];
                let concentration = sample.substances[self.substance_idx];

                tree.add(
                    pos,
                    concentration
                ).unwrap();
            }
        }

        tree
    }

    /// Builds a kdtree of substance values indexed by their position in UV space
    fn build_position_uv_tree(&self, surf: &Surface, entity_idx: usize) -> KdTree<(Vector3<f32>, Vector2<f32>), [f64; 2]> {
        let mut tree = KdTree::new(2); //KdTree::new_with_capacity(2, surf.samples.len());

        for sample in &surf.samples {
            if sample.entity_idx == entity_idx {
                let pos = [sample.texcoords.x as f64, sample.texcoords.y as f64];
                let position = sample.position;
                let texcoords = sample.texcoords;

                tree.add(
                    pos,
                    (position, texcoords)
                ).unwrap();
            }
        }

        tree
    }

    /// Looks up the surfels within the given radius at the given point in UV space
    /// and calculates a combined substance concentration.
    fn gather_concentration_at(&self, concentrations: &KdTree<f32, [f64; 2]>, u: f32, v: f32, radius: f32) -> f32 {
        let uv = [ u as f64, v as f64 ];
        // REVIEW since we use squared_euclidean, we should use radius^2 ?
        let within_radius = concentrations.within(&uv, (radius*radius) as f64, &squared_euclidean).unwrap();

        let (sample_count, concentration_sum) = within_radius.iter()
            .fold(
                (0_usize, 0.0_f32),
                |(count, concentration_sum), &(_, &concentration)| {
                    (count + 1, concentration_sum + concentration)
                }
            );

        if sample_count > 0 {
            concentration_sum / (sample_count as f32)
        } else {
            //warn!("No sample at UV {:?}", uv);
            NAN
        }
    }
}

impl Vertex for (Vector3<f32>, Vector2<f32>) {
    // Triangles in UV space
    fn position(&self) -> Vector3<f32> {
        Vector3::new(self.1.x, self.1.y, 0.0)
    }
}
