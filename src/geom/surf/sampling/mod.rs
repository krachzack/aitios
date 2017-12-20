mod vtx;

use self::vtx::SparseVertex;

use ::geom::tri::Triangle;
use ::geom::vtx::Vertex;

use ::rand;
use ::rand::Rng;

use ::cgmath::Vector3;

use std::f32;

use ::kdtree::kdtree::Kdtree;

/// Generates a vector of surface samples using dart throwing.
///
/// To create a sample from a chosen surface position, the passed function is invoked.
pub fn throw_darts<I, V, F, S>(triangles: I, minimum_sample_distance: f32, triangle_and_sample_pos_to_sample: F) -> Vec<S>
    where I : IntoIterator<Item = Triangle<V>>,
        V : Vertex,
        F : Fn(&Triangle<V>, Vector3<f32>) -> S
{
    info!("Preparing dart throwing...");

    // This vector is going to be huge but we need the original
    // triangles later for interpolation, maybe this should instead
    // be a reference to an indexed triangle structure
    let fat_triangles : Vec<_> = triangles.into_iter().collect();

    let active_triangles : Vec<_> = fat_triangles.iter()
        .enumerate()
        .map(|(i, f)| Triangle::new(
            SparseVertex {
                mother_triangle_idx: Some(i),
                position: [
                    f.vertices[0].position().x as f64,
                    f.vertices[0].position().y as f64,
                    f.vertices[0].position().z as f64
                ]
            },
            SparseVertex {
                mother_triangle_idx: Some(i),
                position: [
                    f.vertices[1].position().x as f64,
                    f.vertices[1].position().y as f64,
                    f.vertices[1].position().z as f64
                ]
            },
            SparseVertex {
                mother_triangle_idx: Some(i),
                position: [
                    f.vertices[2].position().x as f64,
                    f.vertices[2].position().y as f64,
                    f.vertices[2].position().z as f64
                ]
            }
        ))
        .collect();

    info!("Collected {} active triangles...", active_triangles.len());

    let mut generated_samples = Vec::new();

    let bin_count = 22;
    let mut active_triangles = TriangleBins::new(active_triangles, bin_count);
    // Empty kdtree not allowed, so we use option
    let mut placed_samples : Option<Kdtree<SparseVertex>> = None;
    // Exit condititon, discard fragments with area smaller than this so the active triangles get
    // empty eventually and splitting stops at some point
    let min_fragment_area = (0.5 * minimum_sample_distance) * (0.5 * minimum_sample_distance) * f32::consts::PI;

    info!("Throwing darts on {} active triangles with a minimum sample distance of {}...", active_triangles.triangle_count(), minimum_sample_distance);
    while active_triangles.triangle_count() > 20 {
        let tri = active_triangles.sample_triangle();
        let candidate_point = tri.sample_vertex();

        let meets_minimum_distance_requirement = {
            if let Some(placed_samples) = placed_samples.as_ref() {
                !placed_samples.has_neighbor_in_range(&candidate_point, minimum_sample_distance as f64)
            } else {
                // No point added yet, good to go for the first point
                true
            }
        };

        if meets_minimum_distance_requirement {
            if let None = placed_samples {
                let mut points = [ candidate_point ];
                placed_samples = Some(Kdtree::new(&mut points));
            } else {
                if let Some(placed_samples) = placed_samples.as_mut() {
                    placed_samples.insert_nodes_and_rebuild(&mut [ candidate_point ]);
                }
            }

            generated_samples.push(
                triangle_and_sample_pos_to_sample(
                    &fat_triangles[candidate_point.mother_triangle_idx.unwrap()],
                    candidate_point.position()
                )
            );

            trace!("Generated {} samples, {} triangles left...", generated_samples.len(), active_triangles.triangle_count());
        }

        let is_covered = |tri : &Triangle<SparseVertex>| {
            let center = tri.center();
            let center = SparseVertex {
                mother_triangle_idx: None,
                position: [center.x as f64, center.y as f64, center.z as f64]
            };

            match placed_samples.as_ref() {
                Some(placed_samples) => tri.is_inside_sphere(
                    // Search for nearest point to the center of the triangle
                    placed_samples.nearest_search(&center).position(),
                    0.5 * minimum_sample_distance
                ),
                None => false
            }
        };

        if !is_covered(&tri) {
            tri.split_at_edge_midpoints()
                .iter()
                .filter(|t| t.area() > min_fragment_area && !is_covered(t))
                .cloned()
                .for_each(|t| active_triangles.push(t));

        }
    }

    info!("Done throwing darts!");

    generated_samples
}

struct TriangleBins {
    bins: Vec<Vec<Triangle<SparseVertex>>>,
    bin_areas: Vec<f32>,
    bin_areas_sum: f32,
    first_bin_max_area: f32
}

impl TriangleBins {
    fn new(
        triangles: Vec<Triangle<SparseVertex>>,
        bin_count: usize
    ) -> TriangleBins {

        let (bins, first_bin_max_area) = partition_triangles(triangles, bin_count);
        let bin_areas : Vec<f32> = bins.iter()
            .map(|b| {
                b.iter()
                    .map(|t| t.area())
                    .sum()
            })
            .collect();

        let bin_areas_sum = bin_areas.iter()
            .sum();

        TriangleBins { bins, bin_areas, bin_areas_sum, first_bin_max_area }
    }

    fn bin_max_area(&self, of_bin_with_idx: usize) -> f32 {
        self.first_bin_max_area * (2.0 as f32).powi(-(of_bin_with_idx as i32))
    }

    fn push(&mut self, tri: Triangle<SparseVertex>) {
        let area = tri.area();
        assert!(area <= self.first_bin_max_area, "{}", self.bins.iter()
            .map(|b| b.len())
            .sum::<usize>());
        let bin_idx = (self.first_bin_max_area / tri.area()).log2() as usize;
        if bin_idx < self.bins.len() {
            self.bins[bin_idx].push(tri);
            self.bin_areas[bin_idx] = self.bins[bin_idx].iter()
                    .map(|t| t.area())
                    .sum();

            self.bin_areas_sum = self.bin_areas.iter().sum();
        }
    }

    /// Samples a random triangle by first selecting a bin with probability proportional
    /// to its area and then selecting a triangle via rejection sampling.
    ///
    /// The sampled triangle is removed from its bin.
    fn sample_triangle(&mut self) -> Triangle<SparseVertex> {
        let bin_idx = self.sample_bin_idx();
        let bin_max_area = self.bin_max_area(bin_idx);
        let mut rng = rand::thread_rng();

        // Rejection sampling, try random and accept with probility proportional
        // to area. This is apparently constant time on average
        loop {
            let random_tri_idx = rng.gen_range(0_usize, self.bins[bin_idx].len());
            let area = self.bins[bin_idx][random_tri_idx].area();
            let acceptance_probability = area / bin_max_area;

            if rng.next_f32() < acceptance_probability {
                let random_tri = self.bins[bin_idx].swap_remove(random_tri_idx);

                self.bin_areas[bin_idx] = self.bins[bin_idx].iter()
                    .map(|t| t.area())
                    .sum();

                self.bin_areas_sum = self.bin_areas.iter().sum();

                return random_tri;
            }
        }
    }

    fn triangle_count(&self) -> usize {
        self.bins.iter()
            .map(|b| b.len())
            .sum()
    }

    /// Samples a random bin index with a probability proportional to the contained triangles area
    fn sample_bin_idx(&self) -> usize {
        let mut r = rand::random::<f32>() * self.bin_areas_sum;

        for (idx, area) in self.bin_areas.iter().enumerate() {
            r -= area;
            if r <= 0.0 {
                return idx;
            }
        }

        panic!("No bin sampled, bin_areas_sum ({}) and bin_areas.sum() ({}) must be out of sync", self.bin_areas_sum, self.bin_areas.iter().sum::<f32>());
    }
}

fn partition_triangles(
    triangles: Vec<Triangle<SparseVertex>>,
    bin_count: usize
) -> (Vec<Vec<Triangle<SparseVertex>>>, f32)
{
    let max_area = triangles.iter()
        .map(|t| t.area())
        .fold(f32::NEG_INFINITY, f32::max);

    let mut bins = Vec::new();
    for _ in 0..bin_count {
        bins.push(Vec::new());
    }

    for triangle in triangles.into_iter() {
        let area = triangle.area();
        let bin_idx = (max_area / area).log2() as usize;

        if bin_idx < bin_count {
            bins[bin_idx].push(triangle);
        } else {
            // During intitial binning, do not filter out very small triangles
            //bins[bin_idx].push(triangle);

            warn!("Ignoring triangle with too small area {} during initial binning", area);
        }
    }

    (bins, max_area)
}

/*fn sample_on_triangle<V>(triangle: &Triangle<V>) -> V
    where V : Vertex + Clone + Mul<f32, Output = V> + Add<V, Output = V>
{
    let weights = {
        let u = rand::random::<f32>();
        let v = rand::random::<f32>();

        let sqrt_u = u.sqrt();

        [
            1.0 - sqrt_u,
            (sqrt_u * (1.0 - v)),
            (sqrt_u * v)
        ]
    };

    triangle.interpolate_vertex_at_bary(weights)
}*/
