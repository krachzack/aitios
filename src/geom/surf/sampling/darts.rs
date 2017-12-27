
use super::vtx::SparseVertex;
use super::bins::TriangleBins;

use ::geom::tri::Triangle;
use ::geom::vtx::Vertex;

use ::kdtree::kdtree::Kdtree;

pub struct Darts {
    active_triangles: TriangleBins,
    min_point_distance: f64,
    previous_samples: Option<Kdtree<SparseVertex>>
}

impl Darts
{
    pub fn new<'a, I, V>(triangles: I, min_point_distance: f32) -> Darts
        where I: IntoIterator<Item = &'a Triangle<V>>,
            V : Vertex + 'a
    {
        trace!("Binning triangles for sampling...");
        let active_triangles = TriangleBins::new(
            as_sparse(triangles),
            32
        );
        trace!("Ok");

        Darts {
            active_triangles,
            previous_samples: None,
            min_point_distance: min_point_distance as f64
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
        true
    }
}

impl Iterator for Darts {
    type Item = SparseVertex;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.active_triangles.triangle_count() == 0 {
                return None;
            }

            let fragment = self.active_triangles.sample_triangle();

            let sample = {
                let sample_candidate = fragment.sample_vertex();
                if self.meets_minimum_distance_requirement(&sample_candidate) {
                    self.add_sample(sample_candidate);
                    Some(sample_candidate)
                } else {
                    None
                }
            };

            if !self.is_covered(&fragment) {
                for sub_fragment in fragment.split_at_edge_midpoints().iter() {
                    if(sub_fragment.area() > 0.000001 && !self.is_covered(sub_fragment)) {
                        self.active_triangles.push(*sub_fragment)
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
            V : Vertex + 'a
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
