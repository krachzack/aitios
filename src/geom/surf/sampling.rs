
use ::geom::tri::Triangle;
use ::geom::vtx::Vertex;

use ::rand;

use ::cgmath::Vector3;

/// Vertex consisting of position and a reference to the triangle that
/// this vertex originated from. By having a small vertex type, we can
/// more cheaply create new triangles.
struct SparseVertex<'a, V : 'a + Vertex> {
    mother_triangle: &'a Triangle<V>,
    position: Vector3<f32>
}

impl<'a, V : Vertex> Vertex for SparseVertex<'a, V> {
    fn position(&self) -> Vector3<f32> {
        self.position
    }
}

pub fn sample<I, V>(triangles: I)
    where I : IntoIterator<Item = Triangle<V>>,
        V : Vertex
{
    // 1. make active list of all triangles (logarithmicly binned by area)
    // 2. initialize empty point set
    // 3. throw darts
    // 3.1 select from active list with probability proportional to area
    // 3.2 choose random point on triangle
    // 3.3 add to point set if random point meets minimum distance requirement
    // 3.4 whether point generated or not, check if fragment is covered by any single point in the set
    // 3.4.1 If covered, remove from active list
    // 3.4.2 If not covered, remove from active list but split into smaller fragments and add them to active list instead
    // 3.5 terminate if no more active fragment
}

fn sample_on_triangle<V : Vertex>(triangle: Triangle<V>) -> Vector3<f32> {
    let positions = triangle.vertices.iter()
        .map(|v| v.position());

    let weights = {
        let u = rand::random::<f32>();
        let v = rand::random::<f32>();

        [
            1.0 - u.sqrt(),
            (u.sqrt() * (1.0 - v)),
            (u.sqrt() * v)
        ]
    };

    weights.iter()
        .zip(positions)
        .map(|(weight, position)| position * *weight)
        .sum()
}
