use super::*;

use ::cgmath::{Vector2, Vector3};
use ::cgmath::prelude::*;
use ::nearest_kdtree::KdTree;
use ::geom::sampling::{throw_darts, sample_with_density};
use ::geom::scene::Triangle;

use std::collections::HashMap;

pub struct SurfaceBuilder {
    samples: Vec<Surfel>,
    /// Initial deterioration rate of the probability of a gammaton moving further in a straight line
    delta_straight: f32,
    /// Initial deterioration rate of the probability of a gammaton moving in a piecewise approximated parabolic path
    delta_parabolic: f32,
    /// Initial deterioration rate of the probability of a gammaton flowing in a tangent direction
    delta_flow: f32,
    /// Holds the initial amount of substances as numbers in the interval 0..1
    substances: Vec<f32>,
    deposition_rates: Vec<f32>,
    sampling: SurfelSampling,
    material_overrides: HashMap<String, Box<SurfaceBuilder>>
}

#[derive(Copy, Clone)]
enum SurfelSampling {
    /// Examines each triangle and randomly samples an amount of points proporitional to the given
    /// point density per square unit in world space. Clumps together on smaller scale, but crazy fast.alloc
    /// Use `MinimumDistance` for better quality.
    PerSqrUnit(f32),
    /// Uses dart throwing algorithm to generate a poisson disk set with givne minimum distance.
    /// Slower than `PerSqrUnit`, but surfels are more evenly spaced.
    MinimumDistance(f32)
}

impl SurfaceBuilder {
    pub fn new() -> SurfaceBuilder {
        SurfaceBuilder {
            samples: Vec::new(),
            delta_straight: 0.0,
            delta_parabolic: 0.0,
            delta_flow: 0.0,
            substances: Vec::new(),
            deposition_rates: Vec::new(),
            sampling: SurfelSampling::MinimumDistance(0.1),
            material_overrides: HashMap::new()
        }
    }

    pub fn override_material<F, S>(mut self, material_name: S, override_func: F) -> SurfaceBuilder
        where F : FnOnce(SurfaceBuilder) -> SurfaceBuilder, S : Into<String>
    {
        let derived_builder = SurfaceBuilder {
            samples: Vec::new(),
            substances: self.substances.clone(),
            deposition_rates: self.deposition_rates.clone(),
            material_overrides: HashMap::new(),
            ..self
        };

        self.material_overrides.insert(
            material_name.into(),
            Box::new(override_func(derived_builder))
        );

        self
    }

    /// Sets the default delta straight. Can be overriden per material.
    pub fn delta_straight(mut self, delta_straight: f32) -> SurfaceBuilder {
        self.delta_straight = delta_straight;
        self
    }

    /// Sets the default deposition rate if not overridden per material
    pub fn deposition_rates<D>(mut self, deposition_rates: D) -> SurfaceBuilder
        where D : IntoIterator<Item = f32>
    {
        self.deposition_rates = deposition_rates.into_iter().collect();
        self
    }

    #[allow(dead_code)]
    pub fn delta_parabolic(mut self, delta_parabolic: f32) -> SurfaceBuilder {
        self.delta_parabolic = delta_parabolic;
        self
    }

    #[allow(dead_code)]
    pub fn delta_flow(mut self, delta_flow: f32) -> SurfaceBuilder {
        self.delta_flow = delta_flow;
        self
    }

    /// Sets initial material composition of all surfels in the Surface built with this builder.
    pub fn substances(mut self, substances: &Vec<f32>) -> SurfaceBuilder {
        self.substances = substances.clone();
        self
    }

    pub fn min_sample_distance(mut self, min_sample_distance: f32) -> SurfaceBuilder {
        self.sampling = SurfelSampling::MinimumDistance(min_sample_distance);
        self
    }

    pub fn sample_density(mut self, surfels_per_sqr_unit: f32) -> SurfaceBuilder {
        self.sampling = SurfelSampling::PerSqrUnit(surfels_per_sqr_unit);
        self
    }

    /// Creates a surface from only points
    /// Only useful for debugging, since you can make a surface and dump it.
    pub fn add_surface_from_points<P>(mut self, points: P) -> SurfaceBuilder
    where
        P : IntoIterator<Item = Vector3<f32>> {

        let prototype_surfel = Surfel {
            position: Vector3::new(-1.0, -1.0, -1.0),
            normal: Vector3::new(0.0, 0.0, 0.0),
            texcoords: Vector2::new(-1.0, -1.0),
            entity_idx: 0,
            delta_straight: self.delta_straight,
            delta_parabolic: self.delta_parabolic,
            delta_flow: self.delta_flow,
            substances: self.substances.clone(),
            deposition_rates: self.deposition_rates.clone()
        };

        let surfels = points.into_iter()
            .map(
                |position| Surfel {
                    position: position,
                    substances: prototype_surfel.substances.clone(),
                    deposition_rates: prototype_surfel.deposition_rates.clone(),
                    ..prototype_surfel
                }
            );

        self.samples.extend(surfels);
        self
    }

    /// Creates a surface model by sampling a poisson disk set on the surface of the given scene.
    /// The distance between the neighbouring points should be more than min_sample_distance but
    /// smaller than 2 * min_sample_distance.
    ///
    /// The initial values of the surfels are provided to the builder before calling
    /// this method (not after).
    pub fn add_surface_from_scene(mut self, scene: &Scene) -> SurfaceBuilder {
        let boxed_self = Box::new(SurfaceBuilder {
            samples: Vec::new(),
            substances: self.substances.clone(),
            deposition_rates: self.deposition_rates.clone(),
            material_overrides: HashMap::new(),
            ..self
        });

        {
            let builder_per_material : Vec<&Box<SurfaceBuilder>> = {
                let overrides = &self.material_overrides;
                scene.materials.iter()
                    .map(|m| overrides.get(&m.name).unwrap_or(&boxed_self))
                    .collect()
            };

            let make_surfel = |t : &Triangle, position| {
                let material_builder = builder_per_material[t.vertices[0].material_idx];

                let mut texcoords = t.interpolate_at(position, |v| v.texcoords);

                // TODO maybe add warning if UVs degenerate
                if texcoords.x < 0.0 {
                    texcoords.x = 0.0;
                } else if texcoords.x > 1.0 {
                    texcoords.x = 1.0;
                }

                if texcoords.y < 0.0 {
                    texcoords.y = 0.0;
                } else if texcoords.y > 1.0 {
                    texcoords.y = 1.0;
                }

                // TODO this would be a good place to read initital surface properties from a texture

                let normal = t.interpolate_at(position, |v| v.normal);
                let normal = normal.normalize(); // normalize since interpolation can cause distortions

                Surfel {
                    position,
                    normal,
                    texcoords,
                    entity_idx: t.vertices[0].entity_idx,
                    delta_straight: material_builder.delta_straight,
                    delta_parabolic: material_builder.delta_parabolic,
                    delta_flow: material_builder.delta_flow,
                    substances: material_builder.substances.clone(),
                    deposition_rates: material_builder.deposition_rates.clone()
                }
            };

            self.samples.extend(
                match &boxed_self.sampling {
                    &SurfelSampling::MinimumDistance(dist) => throw_darts(scene.triangles(), dist, make_surfel),
                    &SurfelSampling::PerSqrUnit(per_sqr_unit) => sample_with_density(scene.triangles(), per_sqr_unit, make_surfel)
                }
            );
        }

        self
    }

    /// Consumes the builder to create a new surface that is returned.
    pub fn build(self) -> Surface {
        let spatial_idx = {
            let mut tree = KdTree::new(3);

            self.samples.iter()
                .enumerate()
                .for_each(|(idx, s)| tree.add(
                    [s.position.x as f64, s.position.y as f64, s.position.z as f64],
                    idx
                ).unwrap());

            tree
        };

        Surface { samples: self.samples, spatial_idx }
    }
}
