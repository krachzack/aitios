
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

    fn count(&self) -> usize {
        let own_len = self.data.len();
        let child_len : usize = self.children.iter()
            .filter_map(|c| c.as_ref().map(|c| c.count()))
            .sum();

        own_len + child_len
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use ::cgmath::Vector3;

    #[test]
    fn test_octree_count() {
        let tree = Octree::build(vec![
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
        ]);

        assert_eq!(tree.count(), 2);
    }
}
