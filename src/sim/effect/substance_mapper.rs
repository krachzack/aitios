use super::scene::SceneEffect;
use super::substance_map::SubstanceMap;
use super::substance_map_material::SubstanceMapMaterialEffect;

use ::geom::scene::Scene;
use ::geom::surf::Surface;

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
enum Sampling {
    /// Calculates a texture pixel by looking up all surfels within the
    /// given radius in UV space and taking the average
    Radius(f32)
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
    pub fn new(substance_idx: usize, texture_width: usize, texture_height: usize, after_effects: Vec<Box<SubstanceMapMaterialEffect>>) -> SubstanceMapper {
        SubstanceMapper {
            substance_idx,
            sampling: Sampling::Radius(3.0 / (texture_width as f32)), // within three pixels distance in UV space
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
                Sampling::Radius(radius) => self.gather_radius(surf, entity_idx, radius, self.texture_width, self.texture_height)
            }
        )
    }

    fn gather_radius(&self, surf: &Surface, entity_idx: usize, radius: f32, tex_width: usize, tex_height: usize) -> Vec<f32> {
        let mut concentrations = Vec::with_capacity(tex_width * tex_height);

        let concentration_tree = self.build_substance_tree(surf, entity_idx);

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
    fn build_substance_tree(&self, surf: &Surface, entity_idx: usize) -> KdTree<f32, [f64; 2]> {
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
