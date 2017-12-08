
use super::spatial::Spatial;
use super::aabb::Aabb;

use std::cmp::PartialOrd;

pub struct Octree<T>
    where T: Spatial
{
    bounds: Aabb,
    data: Vec<T>,
    children: [Option<Box<Octree<T>>>; 8]
}

impl<T> Octree<T>
    where T: Spatial
{
    // Builds an octree from a given draining iterator over something Spatial
    pub fn build<E>(entities: E) -> Octree<T>
        where E: IntoIterator<Item = T>
    {
        let data : Vec<T> = entities.into_iter().collect();

        let bounds = Aabb::union(
            data.iter()
                .map(|e| e.bounds())
        );

        Octree {
            bounds,
            data,
            children: [None, None, None, None, None, None, None, None]
        }
    }

}