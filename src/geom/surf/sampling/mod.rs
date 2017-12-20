mod vtx;
mod bins;

use self::vtx::SparseVertex;
use self::bins::TriangleBins;

use ::geom::tri::Triangle;
use ::geom::vtx::Vertex;

use ::cgmath::Vector3;

use ::kdtree::kdtree::Kdtree;

use std::f32::consts::PI;

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
    let min_fragment_area = (0.5 * minimum_sample_distance) * (0.5 * minimum_sample_distance) * PI;

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
