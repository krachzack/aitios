mod vtx;
mod bins;
mod darts;

use self::darts::Darts;

use ::geom::tri::Triangle;
use ::geom::vtx::Vertex;

use ::cgmath::Vector3;

use std::time::Instant;


/// Generates a vector of surface samples using dart throwing.
///
/// To create a sample from a chosen surface position, the passed function is invoked.
pub fn throw_darts<I, V, F, S>(triangles: I, minimum_sample_distance: f32, triangle_and_sample_pos_to_sample: F) -> Vec<S>
    where I : IntoIterator<Item = Triangle<V>>,
        V : Vertex,
        F : Fn(&Triangle<V>, Vector3<f32>) -> S
{
    let fat_triangles : Vec<_> = triangles.into_iter().collect();
    let fat_triangle_count = fat_triangles.len();

    info!("Throwing darts with 2r={}...", minimum_sample_distance);
    let start_time = Instant::now();

    let mut sampled = 0;

    let samples = Darts::new(fat_triangles.iter(), minimum_sample_distance)
        .inspect(|_| {
            sampled += 1;
            if sampled % (fat_triangle_count / 5) == 0 {
                info!("{} points sampled...", sampled);
            }
        })
        .map(|v| triangle_and_sample_pos_to_sample(&fat_triangles[v.mother_triangle_idx.unwrap()], v.position()))
        .collect();

    info!("Ok, took {}s", start_time.elapsed().as_secs());

    samples


    /*
    use self::vtx::SparseVertex;
    use self::bins::TriangleBins;
    use ::cgmath::prelude::*;
    use ::kdtree::kdtree::Kdtree;
    use std::f32::consts::PI;

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
    // This buffers nodes waiting to be inserted so we dont have to rebuild the tree as much
    let placed_samples_pending_max_len = 128;
    let mut placed_samples_pending : Vec<SparseVertex> = Vec::with_capacity(placed_samples_pending_max_len);
    // Exit condititon, discard fragments with area smaller than this so the active triangles get
    // empty eventually and splitting stops at some point
    let min_fragment_area = (0.5 * minimum_sample_distance) * (0.5 * minimum_sample_distance) * PI;


    // !!!
    // TODO KEEP LIST OF 128 SAMPLES FOR SLOW DISTANCE CHECK AND ONLY REBUILD TREE WHEN FULL
    // !!!


    info!("Throwing darts on {} active triangles with a minimum sample distance of {}...", active_triangles.triangle_count(), minimum_sample_distance);
    while active_triangles.triangle_count() > 20 {
        let tri = active_triangles.sample_triangle();
        let candidate_point = tri.sample_vertex();

        let meets_minimum_distance_requirement = {
            let meets_in_tree = if let Some(placed_samples) = placed_samples.as_ref() {
                !placed_samples.has_neighbor_in_range(&candidate_point, minimum_sample_distance as f64)
            } else {
                true
            };

            meets_in_tree && placed_samples_pending.iter()
                    .all(|s| (s.position() - candidate_point.position()).magnitude2() > (minimum_sample_distance*minimum_sample_distance))
        };

        if meets_minimum_distance_requirement {
            placed_samples_pending.push(candidate_point);

            if placed_samples_pending.len() == placed_samples_pending_max_len {
                if let None = placed_samples {
                    placed_samples = Some(Kdtree::new(&mut placed_samples_pending));
                } else {
                    if let Some(placed_samples) = placed_samples.as_mut() {
                        placed_samples.insert_nodes_and_rebuild(&mut placed_samples_pending);
                    }
                }
                placed_samples_pending.clear();
                trace!("Generated {} samples, {} triangles left...", generated_samples.len(), active_triangles.triangle_count());
            }

            generated_samples.push(
                triangle_and_sample_pos_to_sample(
                    &fat_triangles[candidate_point.mother_triangle_idx.unwrap()],
                    candidate_point.position()
                )
            );
        }

        let is_covered = |tri : &Triangle<SparseVertex>| {
            let center = tri.center();
            let center = SparseVertex {
                mother_triangle_idx: None,
                position: [center.x as f64, center.y as f64, center.z as f64]
            };

            let covered_in_tree  = match placed_samples.as_ref() {
                Some(placed_samples) => tri.is_inside_sphere(
                    // Search for nearest point to the center of the triangle
                    placed_samples.nearest_search(&center).position(),
                    0.5 * minimum_sample_distance
                ),
                None => false
            };

            covered_in_tree && placed_samples_pending.iter()
                .any(|s| tri.is_inside_sphere(s.position(), 0.5 * minimum_sample_distance))
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

    generated_samples*/
}

