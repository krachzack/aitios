use super::*;

use ::cgmath::{Vector2, Vector3};
use ::kdtree::kdtree::Kdtree;

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
    surfels_per_sqr_unit: f32,
}

impl SurfaceBuilder {
    pub fn new() -> SurfaceBuilder {
        SurfaceBuilder {
            samples: Vec::new(),
            delta_straight: 0.0,
            delta_parabolic: 0.0,
            delta_flow: 0.0,
            substances: Vec::new(),
            surfels_per_sqr_unit: 10000.0
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

    pub fn surfels_per_sqr_unit(mut self, surfels_per_sqr_unit: f32) -> SurfaceBuilder {
        self.surfels_per_sqr_unit = surfels_per_sqr_unit;
        self
    }

    /// Creates a surface from only points
    /// Only useful for debugging, since you can make a surface and dump it.
    pub fn add_surface_from_points<P>(mut self, points: P) -> SurfaceBuilder
    where
        P : IntoIterator<Item = Vector3<f32>> {

        let prototype_surfel = Surfel {
            position: Vector3::new(-1.0, -1.0, -1.0),
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

        let surfels_per_sqr_unit = self.surfels_per_sqr_unit;

        self.samples.extend(
            scene.triangles()
                .flat_map(|t| {
                    let surfel_count = (surfels_per_sqr_unit * t.area()).ceil() as i32;
                    let p0 = t.vertices[0].position;
                    let p1 = t.vertices[1].position;
                    let p2 = t.vertices[2].position;
                    let entity_idx = t.vertices[0].entity_idx;
                    let substances = substances.clone();

                    (0..surfel_count).map(move |_| {
                        let u = rand::random::<f32>();
                        let v = rand::random::<f32>();
                        let position = (1.0 - u.sqrt()) * p0 +
                                        (u.sqrt() * (1.0 - v)) * p1 +
                                        (u.sqrt() * v) * p2;

                        let mut texcoords = t.interpolate_at(
                            position,
                            |v| v.texcoords
                        );

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

                        Surfel {
                            position,
                            texcoords,
                            entity_idx,
                            delta_straight: delta_straight,
                            delta_parabolic: delta_parabolic,
                            delta_flow: delta_flow,
                            substances: substances.clone()
                        }
                    })
                })
        );

        self
    }

    /// Consumes the builder to create a new surface that is returned.
    pub fn build(self) -> Surface {
        let spatial_idx = {
            let mut spatial_index_data : Vec<_> = self.samples.iter()
                .enumerate()
                .map(|(idx, s)| {
                    let idx = Some(idx);
                    let x = s.position.x as f64;
                    let y = s.position.y as f64;
                    let z = s.position.z as f64;
                    SurfelIndex { idx, position: [x, y, z] }
                })
                .collect();

            Kdtree::new(&mut spatial_index_data)
        };

        Surface { samples: self.samples, spatial_idx }
    }
}
