use ::geom::tri::Triangle;
use ::geom::vtx::Position;

use ::cgmath::Vector3;

/// Generates a vector of surface samples by traversing the given triangle and placing an
/// amount of random surfels proportional to the area of each triangle.
///
/// To create a sample from a chosen surface position, the passed function is invoked.
pub fn sample_with_density<I, V, F, S>(triangles: I, surfels_per_sqr_unit: f32, triangle_and_sample_pos_to_sample: F) -> Vec<S>
    where I : IntoIterator<Item = Triangle<V>>,
        V : Position,
        F : Fn(&Triangle<V>, Vector3<f32>) -> S
{
    assert!(surfels_per_sqr_unit > 0.0);

    let mut samples = Vec::new();

    for tri in triangles.into_iter() {
        let amount = (tri.area() * surfels_per_sqr_unit).ceil() as usize;
        (0..amount).for_each(|_| samples.push(triangle_and_sample_pos_to_sample(&tri, tri.sample_position())));
    }

    samples
}

