
use super::spatial::Spatial;
use super::aabb::Aabb;

use std::iter::FromIterator;

pub struct Octree<T>
    where T: Spatial
{
    bounds: Aabb,
    data: Vec<T>,
    children: [Option<Box<Octree<T>>>; 8]
}

impl<T> Octree<T>
    where T : Spatial
{
    #[cfg(test)]
    fn node_count(&self) -> usize {
        1 + self.children.iter()
            .filter_map(|c| c.as_ref().map(|c| c.node_count()))
            .sum::<usize>()
    }

    #[cfg(test)]
    fn entity_count(&self) -> usize {
        let own_len = self.data.len();
        let child_len : usize = self.children.iter()
            .filter_map(|c| c.as_ref().map(|c| c.entity_count()))
            .sum();

        own_len + child_len
    }
}

impl<T> FromIterator<T> for Octree<T>
    where T : Spatial
{
    /// Builds an octree from a given draining iterator over something Spatial
    fn from_iter<I>(entities: I) -> Octree<T>
        where I: IntoIterator<Item = T>
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

#[cfg(test)]
mod test {

    use super::*;
    use ::cgmath::Vector3;

    #[test]
    fn test_octree_count() {
        let tree : Octree<Aabb> = vec![
            // one around the origin
            Aabb {
                min: Vector3::new(-0.1, -0.1, -0.1),
                max: Vector3::new(0.1, 0.1, 0.1)
            },
            // One large above and translated towards Z
            Aabb {
                min: Vector3::new(-0.3, 1.0, 1.0),
                max: Vector3::new(0.3, 1.6, 2.0)
            }
        ].into_iter().collect();

        assert_eq!(tree.entity_count(), 2);
        assert_eq!(tree.node_count(), 1);
    }
}
