use super::vtx::SparseVertex;

use ::geom::tri::Triangle;

use ::rand;
use ::rand::Rng;

use std::f32;

pub struct TriangleBins {
    bins: Vec<Vec<Triangle<SparseVertex>>>,
    bin_areas: Vec<f32>,
    bin_areas_sum: f32,
    first_bin_max_area: f32
}

impl TriangleBins {
    pub fn new(
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

    pub fn push(&mut self, tri: Triangle<SparseVertex>) {
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
    pub fn sample_triangle(&mut self) -> Triangle<SparseVertex> {
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

    pub fn triangle_count(&self) -> usize {
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
