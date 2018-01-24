mod area_weighted;
mod vtx;
mod darts;

use self::darts::Darts;

use ::geom::tri::Triangle;
use ::geom::vtx::Position;

use ::cgmath::Vector3;

use std::time::Instant;


/// Generates a vector of surface samples using dart throwing.
///
/// To create a sample from a chosen surface position, the passed function is invoked.
pub fn throw_darts<I, V, F, S>(triangles: I, minimum_sample_distance: f32, triangle_and_sample_pos_to_sample: F) -> Vec<S>
    where I : IntoIterator<Item = Triangle<V>>,
        V : Position,
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
}

