use super::*;

use ::cgmath::{Vector2, Vector3};
use ::cgmath::prelude::*;
use ::nearest_kdtree::KdTree;
use super::sampling::throw_darts;

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
    min_sample_distance: f32,
}

impl SurfaceBuilder {
    pub fn new() -> SurfaceBuilder {
        SurfaceBuilder {
            samples: Vec::new(),
            delta_straight: 0.0,
            delta_parabolic: 0.0,
            delta_flow: 0.0,
            substances: Vec::new(),
            min_sample_distance: 0.1
        }
    }

    pub fn delta_straight(mut self, delta_straight: f32) -> SurfaceBuilder {
        self.delta_straight = delta_straight;
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
        self.min_sample_distance = min_sample_distance;
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
            substances: self.substances.clone()
        };

        let surfels = points.into_iter()
            .map(
                |position| Surfel {
                    position: position,
                    substances: prototype_surfel.substances.clone(),
                    ..prototype_surfel
                }
            );

        self.samples.extend(surfels);
        self
    }

    /// Creates a surface model by sampling an amount of random points on each
    /// of the traingles in the given indexed mesh that is proportional to the
    /// area of the individual triangles. This way, the sampling is sort of uniform
    /// but not really.
    ///
    /// The initial values of the surfels are provided to the builder before calling
    /// this method (not after).
    pub fn add_surface_from_scene(mut self, scene: &Scene) -> SurfaceBuilder {
        let delta_straight = self.delta_straight;
        let delta_parabolic = self.delta_parabolic;
        let delta_flow = self.delta_flow;
        let substances = self.substances.clone();

        self.samples.extend(
            throw_darts(
                scene.triangles(),
                self.min_sample_distance,
                |t, position| {
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

                    let normal = t.interpolate_at(position, |v| v.normal);
                    let normal = normal.normalize(); // normalize since interpolation can cause distortions

                    Surfel {
                        position,
                        normal,
                        texcoords,
                        entity_idx: t.vertices[0].entity_idx,
                        delta_straight: delta_straight,
                        delta_parabolic: delta_parabolic,
                        delta_flow: delta_flow,
                        substances: substances.clone()
                    }
                }
            )
        );

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
