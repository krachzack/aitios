
use super::vtx::SparseVertex;

use ::geom::TriangleBins;
use ::geom::tri::Triangle;
use ::geom::vtx::Position;

use ::kdtree::kdtree::Kdtree;

use ::cgmath::Vector3;

use std::f32::consts::PI;

pub struct Darts {
    active_triangles: TriangleBins<SparseVertex>,
    min_point_distance: f64,
    /// Do not split a triangle if the resulting subfragments would have a smaller area than this
    disregard_area: f32,
    previous_samples: Option<Kdtree<SparseVertex>>
}

impl Darts
{
    pub fn new<'a, I, V>(triangles: I, min_point_distance: f32) -> Darts
        where I: IntoIterator<Item = &'a Triangle<V>>,
            V : Position + 'a
    {
        trace!("Binning triangles for sampling...");
        let active_triangles = TriangleBins::new(
            as_sparse(triangles),
            32
        );
        trace!("Ok");

        // disregardiness is a factor for running time vs amount of generated points
        // A value closer to zero will result in a more even spacing of points by
        // generating more points and making the poisson disk set more maximal.
        // A higher value will decrease the number of performed triangle splits
        // and thus improve running time. The set will be less evenly spaced though.
        let disregardiness = 0.3;
        let disregard_area = disregardiness * min_point_distance * min_point_distance * PI;

        Darts {
            active_triangles,
            previous_samples: None,
            min_point_distance: min_point_distance as f64,
            disregard_area
        }
    }

    fn meets_minimum_distance_requirement(&self, vtx: &SparseVertex) -> bool {
        if let Some(ref previous_samples) = self.previous_samples {
            !previous_samples.has_neighbor_in_range(vtx, self.min_point_distance)
        } else {
            true
        }
    }

    fn add_sample(&mut self, vtx: SparseVertex) {
        if let Some(ref mut previous_samples) = self.previous_samples {
            previous_samples.insert_node(vtx);
        } else {
            self.previous_samples = Some(Kdtree::new(&mut [ vtx ]));
        }
    }

    fn is_covered(&self, fragment: &Triangle<SparseVertex>) -> bool {
        if let Some(ref previous_samples) = self.previous_samples {
            let proposed_covering_point = fragment.minimum_bounding_sphere_center();

            let nearest = previous_samples.nearest_search(&SparseVertex {
                mother_triangle_idx: None,
                position: [proposed_covering_point.x as f64, proposed_covering_point.y as f64, proposed_covering_point.z as f64]
            });
            let nearest_position = Vector3::new(
                nearest.position[0] as f32,
                nearest.position[1] as f32,
                nearest.position[2] as f32
            );

            fragment.is_inside_sphere(nearest_position, 0.5 * self.min_point_distance as f32)
        } else {
            false
        }
    }
}

impl Iterator for Darts {
    type Item = SparseVertex;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.active_triangles.triangle_count() == 0 {
                return None;
            }

            let fragment = self.active_triangles.pop();

            let sample = {
                let sample_candidate = fragment.sample_vertex();
                if self.meets_minimum_distance_requirement(&sample_candidate) {
                    self.add_sample(sample_candidate);
                    Some(sample_candidate)
                } else {
                    None
                }
            };

            let split_area = 0.25 * fragment.area();
            if split_area > self.disregard_area {
                if !self.is_covered(&fragment) {
                    for sub_fragment in fragment.split_at_edge_midpoints().iter() {
                        if !self.is_covered(sub_fragment) {
                            self.active_triangles.push(*sub_fragment)
                        }
                    }
                }
            }

            if let Some(_) = sample {
                return sample;
            }
        }
    }
}

fn as_sparse<'a, I, V>(triangles: I) -> Vec<Triangle<SparseVertex>>
    where I: IntoIterator<Item = &'a Triangle<V>>,
            V : Position + 'a
{
    triangles.into_iter()
        .enumerate()
        .map(|(idx, t)| {
            let p0 = t.vertices[0].position();
            let p0 = [ p0.x as f64, p0.y as f64, p0.z as f64 ];
            let p1 = t.vertices[1].position();
            let p1 = [ p1.x as f64, p1.y as f64, p1.z as f64 ];
            let p2 = t.vertices[2].position();
            let p2 = [ p2.x as f64, p2.y as f64, p2.z as f64 ];
            let mother_triangle_idx = Some(idx);

            Triangle::new(
                SparseVertex {
                    mother_triangle_idx,
                    position: p0
                },
                SparseVertex {
                    mother_triangle_idx,
                    position: p1
                },
                SparseVertex {
                    mother_triangle_idx,
                    position: p2
                }
            )
        })
        .collect()
}
