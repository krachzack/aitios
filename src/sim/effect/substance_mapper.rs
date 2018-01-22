use super::scene::SceneEffect;
use super::substance_map::SubstanceMap;
use super::substance_map_material::SubstanceMapMaterialEffect;

use ::geom::scene::{Scene, Entity};
use ::geom::surf::Surface;
use ::geom::tri::Triangle;
use ::geom::vtx::{Position, Texcoords};
use ::geom::raster::Rasterize;

use ::cgmath::{Vector2, Vector3};

use ::nearest_kdtree::KdTree;
use ::nearest_kdtree::distance::squared_euclidean;

use std::f32;
use std::f32::{NAN, NEG_INFINITY};
use std::time::Instant;
use std::path::{Path, PathBuf};

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
    fn perform_after_iteration(&self, scene: &mut Scene, surf: &Surface, base_output_prefix: &Path) {
        let mut base_output_prefix = PathBuf::from(base_output_prefix);
        let base_filename = String::from(base_output_prefix.file_name().unwrap().to_str().unwrap());

        let start = Instant::now();

        for entity_idx in 0..scene.entities.len() {
            info!("Gathering {}x{} substance {} map for entity {}...", self.texture_width, self.texture_height, self.substance_idx, scene.entities[entity_idx].name);
            let substance_tex = self.gather(scene, surf, entity_idx);
            info!("Ok, took {}s", start.elapsed().as_secs());

            for (effect_idx, effect) in self.after_effects.iter().enumerate() {
                let prefix = format!("{}-{}-{}-effect-{}", base_filename, entity_idx, scene.entities[entity_idx].name, effect_idx);
                base_output_prefix.push(prefix);

                if let Some(new_material) = effect.perform(&scene.entities[entity_idx], &scene.materials[scene.entities[entity_idx].original_material_idx], &substance_tex, &base_output_prefix) {
                    let new_material_idx = scene.materials.len();
                    scene.materials.push(new_material);
                    scene.entities[entity_idx].material_idx = new_material_idx;
                }

                base_output_prefix.pop();
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

    fn gather(&self, scene: &Scene, surf: &Surface, entity_idx: usize) -> SubstanceMap {
        SubstanceMap::new(
            self.texture_width,
            self.texture_height,
            self.substance_idx,
            entity_idx,
            match self.sampling {
                Sampling::UvRadius(radius) => self.gather_uv_radius(surf, entity_idx, radius, self.texture_width, self.texture_height),
                Sampling::SpaceRadius(radius) => self.gather_space_radius(&scene.entities[entity_idx], surf, radius, self.texture_width, self.texture_height)
            }
        )
    }

    fn gather_space_radius(&self, ent: &Entity, surf: &Surface, _radius_: f32, tex_width: usize, tex_height: usize) -> Vec<f32> {
        info!("Rendering concentrations...");

        let mut concentrations = vec![NAN; tex_width * tex_height];

        ent.triangles()
            // Transform triangles into uv space scaled for target texture dimensions
            .map(|t| Self::to_padded_uv_space(&t, tex_width, tex_height, 4))
            .for_each(|t| t.rasterize(tex_width, tex_height, |x, y| {
               let world_position = t.interpolate_at(Vector3::new(x as f32, y as f32, 0.0), |v| v.0);
               let surfels = surf.nearest_n(world_position, 4);

               let sample_radius = surfels.iter()
                        .map(|&(dist, _)| dist)
                        .fold(NEG_INFINITY, f32::max);

                // This is inspired by photon mapping, see: https://graphics.stanford.edu/courses/cs348b-00/course8.pdf
                // > 1, characterizes the filter
                let k = 2.7;

                let concentration = surfels.iter()
                    .map(|&(dist, surfel)| (1.0 - (dist / (k * sample_radius))) * surfel.substances[self.substance_idx])
                    .sum::<f32>() / (/*PI * sample_radius * sample_radius*/ surfels.len() as f32);

                concentrations[(tex_height - 1 - y) * tex_width + x] = concentration;
            }));


        info!("Done");

        concentrations
    }

    fn to_padded_uv_space<V : Position + Texcoords>(triangle: &Triangle<V>, tex_width: usize, tex_height: usize, padding: usize) -> Triangle<(Vector3<f32>, Vector2<f32>)> {
        let texcoord0 = triangle.vertices[0].texcoords();
        let texcoord1 = triangle.vertices[1].texcoords();
        let texcoord2 = triangle.vertices[2].texcoords();

        let worldpos0 = triangle.vertices[0].position();
        let worldpos1 = triangle.vertices[1].position();
        let worldpos2 = triangle.vertices[2].position();

        // Position in scaled image space
        let mut image_pos0 = Vector2::new(texcoord0.x * (tex_width as f32), (1.0 - texcoord0.y) * (tex_height as f32));
        let mut image_pos1 = Vector2::new(texcoord1.x * (tex_width as f32), (1.0 - texcoord1.y) * (tex_height as f32));
        let mut image_pos2 = Vector2::new(texcoord2.x * (tex_width as f32), (1.0 - texcoord2.y) * (tex_height as f32));

        let image_center = (1.0 / 3.0) * (image_pos0 + image_pos1 + image_pos2);

        if image_pos0.x < image_center.x {
            image_pos0.x -= padding as f32;
        } else if image_pos0.x > image_center.x {
            image_pos0.x += padding as f32;
        }

        if image_pos1.x < image_center.x {
            image_pos1.x -= padding as f32;
        } else if image_pos1.x > image_center.x {
            image_pos1.x += padding as f32;
        }

        if image_pos2.x < image_center.x {
            image_pos2.x -= padding as f32;
        } else if image_pos2.x > image_center.x {
            image_pos2.x += padding as f32;
        }

        if image_pos0.y < image_center.y {
            image_pos0.y -= padding as f32;
        } else if image_pos0.y > image_center.y {
            image_pos0.y += padding as f32;
        }

        if image_pos1.y < image_center.y {
            image_pos1.y -= padding as f32;
        } else if image_pos1.y > image_center.y {
            image_pos1.y += padding as f32;
        }

        if image_pos2.y < image_center.y {
            image_pos2.y -= padding as f32;
        } else if image_pos2.y > image_center.y {
            image_pos2.y += padding as f32;
        }

        Triangle::new(
            // Note might need to change order so the front side is up
            (worldpos0, image_pos0),
            (worldpos1, image_pos1),
            (worldpos2, image_pos2)
        )
    }

    /*#[allow(unused_variables)]
    fn gather_space_radius(&self, ent: &Entity, surf: &Surface, radius: f32, tex_width: usize, tex_height: usize) -> Vec<f32> {
        let mut concentrations = Vec::with_capacity(tex_width * tex_height);

        let tri_tree = self.build_triangle_uv_tree(ent);

        // width and height of a pixel in UV space
        let pixel_width = 1.0 / (tex_width as f64);
        let pixel_height = 1.0 / (tex_height as f64);

        let mut v = 0.5 * pixel_height;

        for _ in 0..tex_height {
            let mut u = 0.5 * pixel_width;

            for _ in 0..tex_width {
                // TODO alternative strategy
                // store triangle centers in UV space, look up nearest triangles
                // and select the first triangle that contains at least one edge of the texel in uv coordinates,
                // or if no triangle contains it, select the closest triangle for interpolation (or drop it?)
                // then, translate the UVs to barycentric coordinates and use them to
                // interpolate a position

                // See: https://answers.unity.com/questions/374778/how-to-convert-pixeluv-coordinates-to-world-space.html

                // Position approximated by looking up the 3 nearest surfels in UV space
                // and then forming a UV-space triangle, then, the position on the given uv
                // coordinate is approximated from barycentric coordinates calculated for the
                // desired uv position within the uv triangle and using them to synthesize a position

                // Find the 4 triangles with the nearest centers in UV space
                let interpolated_position = tri_tree.nearest(
                    &[u, v],
                    4,
                    &squared_euclidean
                ).unwrap()
                    .iter()
                    // Select the triangle reference from the tuple
                    .map(|t| t.1)
                    // Calculate barys in UV space
                    .map(|t| (t, t.barycentric_at(Vector3::new(u as f32, v as f32, 0.0))) )
                    // select the first triangle where the barycentric coordinates are inside
                    /*.find(|&(_, bary)|
                        if bary[1] + bary[2] > 1.0 {
                            false
                        } else if bary[1] < 0.0 {
                            false
                        } else if bary[2] < 0.0 {
                            false
                        } else {
                            true
                        }
                    )
                    // and if found, interpolate the position based on the synthesized UV coordinates
                    */
                    .min_by_key(|&(_, bary)| {
                        let mut error = 0.0;
                        if bary[1] + bary[2] > 1.0 {
                            error += bary[0];
                        } else if bary[1] < 0.0 {
                            error -= bary[1];
                        } else if bary[2] < 0.0 {
                            error -= bary[2];
                        }
                        (error * 1_000_000_000.0) as u64
                    })
                    .map(|(tri, bary)| tri.interpolate_bary(bary, |v| v.0));

                let mut concentration = if let Some(position) = interpolated_position {
                    let surfels = surf.nearest_n(position, 4);

                    let sample_radius = surfels.iter()
                        .map(|&(dist, _)| dist)
                        .fold(NEG_INFINITY, f32::max);

                    // This is enspired by photon mapping, see: https://graphics.stanford.edu/courses/cs348b-00/course8.pdf

                    // > 1, characterizes the filter
                    let k = 2.7;

                    let concentration = surfels.iter()
                        .map(|&(dist, surfel)| (1.0 - (dist / (k * sample_radius))) * surfel.substances[self.substance_idx])
                        .sum::<f32>() / (/*PI * sample_radius * sample_radius*/ surfels.len() as f32);

                    Some(concentration)

                    /*let surfels = surf.find_within_sphere(position, radius);

                    if surfels.is_empty() {
                        //warn!("Could not find surfels near position {:?}", position);
                        None
                    } else {
                        /*let val = surfels.iter()
                            .map(|s| s.substances[self.substance_idx])
                            .sum::<f32>() / (surfels.len() as f32);*/

                        let val = surfels.iter()
                            .map(|s| s.substances[self.substance_idx])
                            .sum::<f32>() / (surfels.len() as f32);

                        Some(val)
                    }*/
                } else {
                    warn!("Could not translate UV ({}/{}) to a position", u, v);
                    None
                };

                concentrations.push(concentration.unwrap_or(NAN));

                u += pixel_width;
            }
            v += pixel_height;
        }

        concentrations
    }*/

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
    /*fn build_triangle_uv_tree(&self, entity: &Entity) -> KdTree<Triangle<(Vector3<f32>, Vector2<f32>)>, [f64; 2]> {
        let mut tree = KdTree::new(2); //KdTree::new_with_capacity(2, surf.samples.len());

        for tri in entity.triangles() {
            if tri.area() > EPSILON {
                let world_center = tri.center();
                let tex_center = tri.interpolate_at(world_center, |v| v.texcoords);
                let tex_center = [ tex_center.x as f64, tex_center.y as f64 ];
                let tri = Triangle::new(
                    (tri.vertices[0].position, tri.vertices[0].texcoords),
                    (tri.vertices[1].position, tri.vertices[1].texcoords),
                    (tri.vertices[2].position, tri.vertices[2].texcoords)
                );

                tree.add(tex_center, tri).unwrap();
            }
        }

        tree
    }*/

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

impl Position for (Vector3<f32>, Vector2<f32>) {
    // Triangles in UV space
    fn position(&self) -> Vector3<f32> {
        Vector3::new(self.1.x, self.1.y, 0.0)
    }
}
