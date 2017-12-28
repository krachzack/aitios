use super::vtx::SparseVertex;

use ::geom::tri::Triangle;

use ::rand;
use ::rand::Rng;

use std::f32;

use ::float_extras::f64::ilogb;

pub struct TriangleBins {
    bins: Vec<Vec<Triangle<SparseVertex>>>,
    /// One over the value of one area unit in bin_areas and bin_areas_sum
    inv_area_quantum: f32,
    /// Approximate area of all binned triangles represented as a multiple of area_quantum
    /// When summing up triangles, their areas will be ceiled.
    /// Adding and subtracting integers is fast and repeatable, while these operations in
    /// floating point introduce some error that eventually sums up to a large error.
    bin_areas_sum: u64,
    /// Approximate areas of bins as multiples of area_quantum, sum of this vector must be
    /// exactly equal to bin_areas_sum
    bin_areas: Vec<u64>,
    /// Contains the upper bounds for triangles stored in a bin
    bin_max_areas: Vec<f32>,
    triangle_count: usize
}

impl TriangleBins {
    pub fn new(
        triangles: Vec<Triangle<SparseVertex>>,
        bin_count: usize
    ) -> TriangleBins {

        let (bins, first_bin_max_area) = partition_triangles(triangles, bin_count);

        let bin_max_areas : Vec<f32> = (0..bin_count)
            .map(|bin_idx| first_bin_max_area * (2.0 as f32).powi(-(bin_idx as i32)))
            .collect();

        // This leads to the smallest possible triangle having size 50,
        // and largest having size 0.01 * 2^(bin_count-1), which is 21_474_836.48 for 32 bins
        let inv_area_quantum = 1.0 / (0.01 * bin_max_areas[bin_count-1]);

        let bin_areas : Vec<u64> = bins.iter()
            .map(|b| {
                b.iter()
                    .map(|t| {
                        (inv_area_quantum * t.area()).ceil() as u64
                    })
                    // Verify no area is zero
                    .inspect(|a| assert_ne!(0_u64, *a))
                    .sum()
            })
            .collect();

        let bin_areas_sum = bin_areas.iter()
            .sum();

        let triangle_count = bins.iter()
            .map(|b| b.len())
            .sum();

        debug!("Bin max areas {:?}", bin_max_areas);
        debug!("Bin areas {:?}", bin_areas);

        TriangleBins { bins, inv_area_quantum, bin_areas, bin_max_areas, bin_areas_sum, triangle_count }
    }

    fn integral_area(&self, float_approximation: f32) -> u64 {
        assert!(float_approximation > 0.0);
        let area = (float_approximation * self.inv_area_quantum).ceil() as u64;
        assert!(area > 0);
        area
    }

    fn bin_max_area(&self, of_bin_with_idx: usize) -> f32 {
        self.bin_max_areas[of_bin_with_idx]
    }

    fn bin_idx_by_area(&self, area: f32) -> usize {
        let first_bin_max_area = self.bin_max_areas[0];
        assert!(area <= first_bin_max_area, "Cannot push triangle with larger area than the largest triangle");
        // faster version of (first_bin_max_area / area).log2()
        ilogb((first_bin_max_area / area) as f64) as usize
    }

    pub fn push(&mut self, tri: Triangle<SparseVertex>) {
        let area = tri.area();
        let bin_idx =  self.bin_idx_by_area(area);
        if bin_idx < self.bins.len() {
            let area = self.integral_area(area);

            self.bins[bin_idx].push(tri);
            self.bin_areas[bin_idx] += area;
            self.bin_areas_sum += area;
            self.triangle_count += 1;
        }
    }

    /// Samples a random triangle by first selecting a bin with probability proportional
    /// to its area and then selecting a triangle via rejection sampling.
    ///
    /// The sampled triangle is removed from its bin.
    pub fn sample_triangle(&mut self) -> Triangle<SparseVertex> {
        assert!(self.bin_areas_sum > 0, "Can only sample triangle with remaining non-empty triangle bins. triangle_count={} bin_areas_sum={} bin_areas={:?}", self.triangle_count, self.bin_areas_sum, self.bin_areas);

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

                let area = self.integral_area(random_tri.area());
                self.bin_areas[bin_idx] -= area;
                self.bin_areas_sum -= area;
                self.triangle_count -= 1;

                return random_tri;
            }
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangle_count
    }

    /// Samples a random bin index with a probability proportional to the contained triangles area
    fn sample_bin_idx(&mut self) -> usize {
        let mut rng = rand::thread_rng();
        let mut r = rng.gen_range(0, self.bin_areas_sum);

        for (idx, area) in self.bin_areas.iter().enumerate() {
            if r < *area {
                if self.bins[idx].is_empty() {
                    panic!("Empty bin sampled, bin_areas_sum ({}) and bin_areas.sum() ({}) must be out of sync, retrying with refreshed areas", self.bin_areas_sum, self.bin_areas.iter().sum::<u64>());
                }

                return idx;
            }

            r -= area;
        }

        panic!("No bin sampled, bin_areas_sum ({}) and bin_areas.sum() ({}) must be out of sync, retrying with refreshed areas, r={}", self.bin_areas_sum, self.bin_areas.iter().sum::<u64>(), r);
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

        if area > 0.0 {
            let bin_idx = bin_idx_by_area(max_area, area);

            if bin_idx < bin_count {
                bins[bin_idx].push(triangle);
            } else {
                // During intitial binning, do not filter out very small triangles
                //bins[bin_idx].push(triangle);

                warn!("Ignoring triangle with too small area {} during initial binning", area);
            }
        } else {
            warn!("Ignoring triangle with area of zero during initial binning");
        }
    }

    (bins, max_area)
}

fn bin_idx_by_area(max_area: f32, area: f32) -> usize {
    assert!(area <= max_area, "Cannot push triangle with larger area than the largest triangle");
    // faster version of (first_bin_max_area / area).log2()
    ilogb((max_area / area) as f64) as usize
}
