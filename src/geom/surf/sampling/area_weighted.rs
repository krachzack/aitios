
use ::geom::TriangleBins;

use ::geom::vtx::Position;

use std::ops::{Mul, Add};

struct AreaWeightedSampler<V>
    where V: Position + Clone + Mul<f32, Output = V> + Add<V, Output = V>
{
    bins: TriangleBins<V>
}

impl<V> Iterator for AreaWeightedSampler<V>
    where V: Position + Clone + Mul<f32, Output = V> + Add<V, Output = V>
{
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.bins.sample().sample_vertex())
    }
}
