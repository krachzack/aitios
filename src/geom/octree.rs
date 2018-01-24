//! Implements octrees composed of objects in 3D space

use std::iter::FromIterator;
use std::mem;

use ::cgmath::Vector3;
use ::cgmath::prelude::Zero;

use super::spatial::Spatial;
use super::aabb::Aabb;
use super::intersect::IntersectRay;

#[derive(Debug)]
pub struct Octree<T>
    where T: Spatial
{
    bounds: Aabb,
    data: Vec<T>,
    children: [Option<Box<Octree<T>>>; 8]
}

fn octants(aabb: &Aabb) -> [Aabb; 8] {
    let &Aabb { min, max } = aabb;
    let dims = max - min;
    let center = min + 0.5 * dims;

    [
        // 0: left bottom back
        Aabb { min: min, max: center },
        // 1: right bottom back
        Aabb {
            min: Vector3::new(center.x, min.y, min.z),
            max: Vector3::new(max.x, center.y, center.z)
        },
        // 2: right bottom front
        Aabb {
            min: Vector3::new(center.x, min.y, center.z),
            max: Vector3::new(max.x, center.y, max.z)
        },
        // 3: left bottom front
        Aabb {
            min: Vector3::new(min.x, min.y, center.z),
            max: Vector3::new(center.x, center.y, max.z)
        },
        // 4: left top back
        Aabb {
            min: Vector3::new(min.x, center.y, min.z),
            max: Vector3::new(center.x, max.y, center.z)
        },
        // 5: right top back
        Aabb {
            min: Vector3::new(center.x, center.y, min.z),
            max: Vector3::new(max.x, max.y, center.z)
        },
        // 6: right top front
        Aabb { min: center, max: max },
        // 7: left top front
        Aabb {
            min: Vector3::new(min.x, center.y, center.z),
            max: Vector3::new(center.x, max.y, max.z)
        }
    ]
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

    #[cfg(test)]
    fn depth(&self) -> usize {
        1 + self.children.iter()
            .map(|c| match c {
                &Some(ref c) => c.depth(),
                _ => 0
            })
            .max()
            .unwrap_or(0)
    }

    fn from_vec_with_bounds(mut own_data: Vec<T>, own_bounds: Aabb, min_node_volume: f32) -> Octree<T> {
        assert!(min_node_volume > 0.0, "When building octree, minimum node volume has to be > 0");

        let mut children = [
            None, None, None, None,
            None, None, None, None
        ];

        // Continue subdividing as long as splitting makes sense and the octants are larger than 0.1 cubic units
        if own_data.len() > 1 && own_bounds.volume() >= min_node_volume {
            let mut child_data : [Vec<T>; 8] = [
                Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                Vec::new(), Vec::new(), Vec::new(), Vec::new()
            ];
            let mut child_octants = octants(&own_bounds);

            let src_data = own_data;
            own_data = Vec::new();

            for ent in src_data {
                // If ent is completely inside aabb of child octant, it gets moved to the child
                // and set to None here
                // If no move ocurred, stays Some(ent) and will instead be moved to own_data

                let mut ent = Some(ent);

                for (ref mut data, octant) in child_data.iter_mut().zip(child_octants.iter()) {
                    let iter_ent = ent.take().unwrap();
                    if octant.is_aabb_inside(&iter_ent.bounds()) {
                        data.push(iter_ent);
                        break;
                    } else {
                        ent = Some(iter_ent);
                    }
                }

                if let Some(ent) = ent {
                    own_data.push(ent);
                }
            }

            for i in 0..children.len() {
                if child_data[i].len() > 0 {
                    children[i] = Some(
                        Box::new(
                            Self::from_vec_with_bounds(
                                // Move values out of arrays and replace them with unit values
                                mem::replace(&mut child_data[i], vec![]),
                                mem::replace(&mut child_octants[i], Aabb { min: Vector3::zero(), max: Vector3::zero() }),
                                min_node_volume
                            )
                        )
                    )
                }
            }
        }

        Octree {
            bounds: own_bounds,
            data: own_data,
            children
        }
    }
}

impl<T> Octree<T>
    where T : Spatial + IntersectRay
{
    pub fn ray_intersection_target_and_parameter(&self, ray_origin: Vector3<f32>, ray_direction: Vector3<f32>) -> Option<(&T, f32)> {
        let mut t_min = None;

        if !self.bounds.intersects_ray(ray_origin, ray_direction) {
            return None;
        }

        // Try intersecting with data in this node
        for data in &self.data {
            if let Some(t) = data.ray_intersection_parameter(ray_origin, ray_direction) {
                if let Some((min_data, min_param)) = t_min.take() {
                    t_min = Some(if t < min_param {
                        (data, t)
                    } else {
                        (min_data, min_param)
                    });
                } else {
                    t_min = Some((data, t));
                }
            }
        }

        // Then, try children
        for child in &self.children {
            if let &Some(ref child) = child {
                if let Some((data, t)) = child.ray_intersection_target_and_parameter(ray_origin, ray_direction) {
                    t_min = Some(match t_min {
                        Some((min_data, t_min)) => if t < t_min { (data, t) } else { (min_data, t_min) },
                        None => (data, t)
                    });
                }
            }
        }

        t_min
    }

    /// Like a ray intersection but with limited range for approximating parabolas
    pub fn line_segment_intersection_target_and_parameter(&self, origin: Vector3<f32>, direction: Vector3<f32>, range: f32) -> Option<(&T, f32)> {
        let mut t_min = None;

        let bounds_intersection_param = self.bounds.ray_intersection_parameter(origin, direction);

        // Assuming direction is normalized, t should be the distance to the intersection point
        // Prune if bounding box is out of range or does not intersect
        if let Some(t) = bounds_intersection_param {
            if t > range {
                return None;
            }
        } else {
            return None;
        }

        for data in &self.data {
            if let Some(t) = data.ray_intersection_parameter(origin, direction) {
                // Ignore out of range hits
                if t <= range {
                    if let Some((min_data, min_param)) = t_min.take() {
                        t_min = Some(if t < min_param {
                            (data, t)
                        } else {
                            (min_data, min_param)
                        });
                    } else {
                        t_min = Some((data, t));
                    }
                }
            }
        }

        for child in &self.children {
            if let &Some(ref child) = child {
                if let Some((data, t)) = child.line_segment_intersection_target_and_parameter(origin, direction, range) {
                    t_min = Some(match t_min {
                        Some((min_data, t_min)) => if t < t_min { (data, t) } else { (min_data, t_min) },
                        None => (data, t)
                    });
                }
            }
        }

        t_min
    }
}

impl<T> FromIterator<T> for Octree<T>
    where T : Spatial
{
    /// Builds an octree from a given draining iterator over something Spatial
    fn from_iter<I>(entities: I) -> Octree<T>
        where I: IntoIterator<Item = T>
    {
        let own_data : Vec<T> = entities.into_iter().collect();
        let own_bounds = Aabb::union(
            own_data.iter()
                .map(|e| e.bounds())
        );

        let min_node_volume = 0.1 * (own_bounds.max.x - own_bounds.min.x);
        Octree::from_vec_with_bounds(own_data, own_bounds, min_node_volume)
    }
}

impl<T> IntersectRay for Octree<T>
    where T : Spatial + IntersectRay
{
    fn ray_intersection_parameter(&self, ray_origin: Vector3<f32>, ray_direction: Vector3<f32>) -> Option<f32> {
        let mut t_min = None;

        if !self.bounds.intersects_ray(ray_origin, ray_direction) {
            return None;
        }

        for data in &self.data {
            if let Some(t) = data.ray_intersection_parameter(ray_origin, ray_direction) {
                t_min = Some(match t_min {
                    Some(t_min) => if t < t_min { t } else { t_min },
                    None => t
                });
            }
        }

        for child in &self.children {
            if let &Some(ref child) = child {
                if let Some(t) = child.ray_intersection_parameter(ray_origin, ray_direction) {
                    t_min = Some(match t_min {
                        Some(t_min) => if t < t_min { t } else { t_min },
                        None => t
                    });
                }
            }
        }

        t_min
    }
}

impl<T> Spatial for Octree<T>
    where T : Spatial
{
    fn bounds(&self) -> Aabb {
        self.bounds
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use ::cgmath::Vector3;
    use super::super::scene::{Scene, Triangle};

    fn make_example_aabb_tree() -> Octree<Aabb> {
        let whole_world = Aabb {
            min: Vector3::new(-10.0, -10.0, -10.0),
            max: Vector3::new(10.0, 10.0, 10.0)
        };

        let around_origin = Aabb {
            min: Vector3::new(-0.1, -0.1, -0.1),
            max: Vector3::new(0.1, 0.1, 0.1)
        };

        let left_top_front1 = Aabb {
            min: Vector3::new(4.9, 4.9, 4.9),
            max: Vector3::new(5.1, 5.1, 5.1),
        };

        let left_top_front2 = Aabb {
            min: Vector3::new(4.9, 4.9, 4.9),
            max: Vector3::new(5.1, 5.1, 5.1),
        };

        let left_top_front3 = Aabb {
            min: Vector3::new(4.9, 4.9, 4.9),
            max: Vector3::new(5.1, 5.1, 5.1),
        };

        vec![
            whole_world,
            around_origin,
            left_top_front1,
            left_top_front2,
            left_top_front3
        ].into_iter().collect()
    }

    fn make_example_aabb_tree_nonoverlapping() -> Octree<Aabb> {
        let whole_world = Aabb {
            min: Vector3::new(-10.0, -10.0, -10.0),
            max: Vector3::new(10.0, 10.0, 10.0)
        };

        let around_origin = Aabb {
            min: Vector3::new(-0.1, -0.1, -0.1),
            max: Vector3::new(0.1, 0.1, 0.1)
        };

        let left_top_front1 = Aabb {
            min: Vector3::new(4.9, 4.9, 4.9),
            max: Vector3::new(5.1, 5.1, 5.1),
        };

        let left_top_front2 = Aabb {
            min: Vector3::new(5.9, 5.9, 5.9),
            max: Vector3::new(6.1, 6.1, 6.1),
        };

        let left_top_front3 = Aabb {
            min: Vector3::new(6.9, 6.9, 6.9),
            max: Vector3::new(7.1, 7.1, 7.1),
        };

        vec![
            around_origin,
            left_top_front1,
            left_top_front2,
            left_top_front3
        ].into_iter().collect()
    }

    #[test]
    fn test_subdivision() {
        let tree = make_example_aabb_tree();

        assert_eq!(tree.depth(), 2);
        assert_eq!(tree.entity_count(), 5);
        assert_eq!(tree.node_count(), 2);

        assert!(
            tree.data.len() == 2 &&
            tree.data.iter().any(|e| e.min.x == -10.0 && e.min.y == -10.0 && e.min.z == -10.0 &&
                                     e.max.x == 10.0 && e.max.y == 10.0 && e.max.z == 10.0) &&
            tree.data.iter().any(|e| e.min.x == -0.1 && e.min.y == -0.1 && e.min.z == -0.1 &&
                                     e.max.x == 0.1 && e.max.y == 0.1 && e.max.z == 0.1),
            "Root node should have whole_world and around_origin, but had data {:?}",
            tree.data.iter()
        );

        assert!(
            tree.children.iter()
                .any(|c| match c {
                    &Some(ref c) =>
                        c.data.len() == 3 &&
                        c.data.iter().all(|e| e.min.x == 4.9 && e.min.y == 4.9 && e.min.z == 4.9 &&
                                              e.max.x == 5.1 && e.max.y == 5.1 && e.max.z == 5.1),
                    &None => false
                }),
            "Expected a direct descendant of the root node to contain three left_top_front, but actual children were: {:?}",
            tree.children
        )
    }

    #[cfg_attr(not(feature = "expensive_tests"), ignore)]
    #[test]
    fn test_large_tree() {
        let scene = Scene::load_from_file("test-scenes/buddha-scene/buddha-scene.obj");
        let tree : Octree<Triangle> = scene.triangles().collect();

        let tree_triangle_count = tree.entity_count();

        assert_eq!(scene.triangle_count(), tree_triangle_count);

        assert!(
            tree.node_count() < (tree_triangle_count / 2),
            "Number of nodes should be significantly smaller than number of triangles, but triangle count was: {} and node count was: {}",
            tree_triangle_count,
            tree.node_count()
        );

        assert!(tree.depth() >= 3, "The tree should be at least 3 levels deep, but was {}", tree.depth());
    }

    #[test]
    fn test_line_segment_intersection() {
        let tree = make_example_aabb_tree_nonoverlapping();

        let intersection = tree.line_segment_intersection_target_and_parameter(Vector3::new(0.0, 10.0, 0.0), Vector3::new(0.0, -1.0, 0.0), 10.0);

        assert!(intersection.is_some());
        if let Some((ref target, ref parameter)) = intersection {
            assert_eq!(*parameter, 9.9, "Expected to hit the AABB centered around the origin, instead hit {:?}", target);
        }

        let intersection = tree.line_segment_intersection_target_and_parameter(Vector3::new(0.0, 10.0, 0.0), Vector3::new(0.0, -1.0, 0.0), 9.8);
        assert!(intersection.is_none());
    }
}
