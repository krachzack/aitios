use super::scene::SceneEffect;
use super::substance_map::SubstanceMap;
use super::substance_map_material::SubstanceMapMaterialEffect;

use ::geom::scene::Scene;
use ::geom::scene::Entity;
use ::geom::surf::Surface;

use ::cgmath::Vector2;
use ::cgmath::prelude::*;

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
        info!("Gathering {}x{} substance map...", self.texture_width, self.texture_height);
        let start = Instant::now();

        for entity_idx in 0..scene.entities.len() {
            let substance_tex = self.gather(surf, entity_idx);

            for effect in &self.after_effects {
                if let Some(new_material) = effect.perform(&scene.entities[entity_idx], &substance_tex) {
                    let new_material_idx = scene.materials.len();
                    scene.materials.push(new_material);
                    scene.entities[entity_idx].material_idx = new_material_idx;
                }
            }
        }

        info!("Ok, took {}s", start.elapsed().as_secs());
    }
}

impl SubstanceMapper {
    pub fn new(substance_idx: usize, texture_width: usize, texture_height: usize, after_effects: Vec<Box<SubstanceMapMaterialEffect>>) -> SubstanceMapper {
        SubstanceMapper {
            substance_idx,
            sampling: Sampling::Radius(0.05),
            texture_width,
            texture_height,
            after_effects
        }
    }

    fn gather(&self, surf: &Surface, entity_idx: usize) -> SubstanceMap {
        SubstanceMap::new(
            self.texture_width,
            self.texture_height,
            match self.sampling {
                Sampling::Radius(radius) => self.gather_radius(surf, entity_idx, radius, self.texture_width, self.texture_height)
            }
        )
    }

    fn gather_radius(&self, surf: &Surface, entity_idx: usize, radius: f32, tex_width: usize, tex_height: usize) -> Vec<f32> {
        let mut concentrations = Vec::with_capacity(tex_width * tex_height);

        // width and height of a pixel in UV space
        let pixel_width = 1.0 / (tex_width as f32);
        let pixel_height = 1.0 / (tex_height as f32);

        let mut u = 0.5 * pixel_width;
        let mut v = 0.5 * pixel_height;

        for _ in 0..tex_width {
            for _ in 0..tex_height {
                concentrations.push(self.gather_concentration_at(surf, entity_idx, u, v, radius));
                v += pixel_width;
            }
            u += pixel_width;
        }

        concentrations
    }

    fn gather_concentration_at(&self, surf: &Surface, entity_idx: usize, u: f32, v: f32, radius: f32) -> f32 {
        let uv = Vector2::new(u, v);
        let (surfel_count, concentration) = surf.iter()
            .filter(|s|
                s.entity_idx == entity_idx &&
                s.texcoords.distance2(uv) < (radius * radius)
            )
            .map(|s| s.substances[self.substance_idx])
            .fold(
                (0_usize, 0.0_f32),
                |(count, concentration_sum), concentration| {
                    (count + 1, concentration_sum + concentration)
                }
            );

        if surfel_count > 0 {
            concentration
        } else {
            NAN
        }
    }
}
