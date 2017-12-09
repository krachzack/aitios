use std::f32::{INFINITY, NEG_INFINITY};
use ::cgmath::Vector3;

/// An axis-aligned bounding box in 3D
#[derive(Debug)]
pub struct Aabb {
    pub min: Vector3<f32>,
    pub max: Vector3<f32>
}

impl Aabb {
    /// Creates the smallest aabb that encloses all of the points returned
    /// by the given iterator.
    /// Returns an aabb with max at negative infinity and min at positive infinity if
    /// the given iterator was empty.
    pub fn from_points<P>(points: P) -> Aabb
        where P: IntoIterator<Item = Vector3<f32>>
    {
        points.into_iter()
            .fold(
                Aabb {
                    min: Vector3::new(INFINITY, INFINITY, INFINITY),
                    max: Vector3::new(NEG_INFINITY, NEG_INFINITY, NEG_INFINITY)
                },
                |Aabb { min, max }, p| {
                    let min_x = if p.x < min.x { p.x } else { min.x };
                    let min_y = if p.y < min.y { p.y } else { min.y };
                    let min_z = if p.z < min.z { p.z } else { min.z };

                    let max_x = if p.x > max.x { p.x } else { max.x };
                    let max_y = if p.y > max.y { p.y } else { max.y };
                    let max_z = if p.z > max.z { p.z } else { max.z };

                    Aabb {
                        min: Vector3::new(min_x, min_y, min_z),
                        max: Vector3::new(max_x, max_y, max_z)
                    }
                }
            )
    }

    /// Returns the smallest aabb that encloses all of the aabb in the given iterator.
    /// Returns an aabb with max at negative infinity and min at positive infinity if
    /// the given iterator was empty.
    pub fn union<A>(aabbs: A) -> Aabb
        where A: IntoIterator<Item = Aabb>
    {
        aabbs.into_iter()
            .fold(
                Aabb {
                    min: Vector3::new(INFINITY, INFINITY, INFINITY),
                    max: Vector3::new(NEG_INFINITY, NEG_INFINITY, NEG_INFINITY)
                },
                |Aabb { min: acc_min, max: acc_max }, Aabb { min: aabb_min, max: aabb_max }| {
                    let min_x = if aabb_min.x < acc_min.x { aabb_min.x } else { acc_min.x };
                    let min_y = if aabb_min.y < acc_min.y { aabb_min.y } else { acc_min.y };
                    let min_z = if aabb_min.z < acc_min.z { aabb_min.z } else { acc_min.z };

                    let max_x = if aabb_max.x > acc_max.x { aabb_max.x } else { acc_max.x };
                    let max_y = if aabb_max.y > acc_max.y { aabb_max.y } else { acc_max.y };
                    let max_z = if aabb_max.z > acc_max.z { aabb_max.z } else { acc_max.z };

                    Aabb {
                        min: Vector3::new(min_x, min_y, min_z),
                        max: Vector3::new(max_x, max_y, max_z)
                    }
                }
            )
    }

    fn is_point_outside(&self, point: Vector3<f32>) -> bool {
        point.x < self.min.x || point.x > self.max.x ||
            point.y < self.min.y || point.y > self.max.y ||
            point.z < self.min.z || point.z > self.max.z
    }

    fn is_point_inside(&self, point: Vector3<f32>) -> bool {
        !self.is_point_outside(point)
    }

    pub fn is_aabb_inside(&self, other: &Aabb) -> bool {
        self.is_point_inside(other.min) && self.is_point_inside(other.max)
    }

    pub fn volume(&self) -> f32 {
        let dims = self.max - self.min;
        dims.x * dims.y * dims.z
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::iter;

    #[test]
    fn test_aabb_from_points_empty() {
        let aabb = Aabb::from_points(iter::empty());

        assert_eq!(aabb.min.x, INFINITY, "Expected infinite AABB from empty points");
        assert_eq!(aabb.min.y, INFINITY, "Expected infinite AABB from empty points");
        assert_eq!(aabb.min.z, INFINITY, "Expected infinite AABB from empty points");

        assert_eq!(aabb.max.x, NEG_INFINITY, "Expected infinite AABB from empty points");
        assert_eq!(aabb.max.y, NEG_INFINITY, "Expected infinite AABB from empty points");
        assert_eq!(aabb.max.z, NEG_INFINITY, "Expected infinite AABB from empty points");
    }

    #[test]
    fn test_aabb_from_single_point() {
        let point = Vector3::new(1.0, 2.0, 3.0);
        let aabb = Aabb::from_points(iter::once(point));

        assert_eq!(aabb.min, point, "Built AABB from single point {:?} and expected min to be equal, but was {:?}", point, aabb.min);
        assert_eq!(aabb.max, point, "Built AABB from single point {:?} and expected max to be equal, but was {:?}", point, aabb.max);
    }

    #[test]
    fn test_aabb_from_points_triangle() {
        let aabb = Aabb::from_points(vec![
            Vector3::new(-0.5, -0.5, 1.0),
            Vector3::new(0.5, -0.5, 1.0),
            Vector3::new(0.0, 0.5, -1.0)
        ]);

        assert_eq!(aabb.min, Vector3::new(-0.5, -0.5, -1.0));
        assert_eq!(aabb.max, Vector3::new(0.5, 0.5, 1.0));
    }

    #[test]
    fn test_inside() {
        let aabb = Aabb::from_points(vec![
            Vector3::new(-0.5, -0.5, 1.0),
            Vector3::new(0.5, -0.5, 1.0),
            Vector3::new(0.0, 0.5, -1.0)
        ]);

        let point = Vector3::new(0.0, 0.0, 0.0);
        assert!(aabb.is_point_inside(point));

        let point = Vector3::new(10.0, 0.0, 0.0);
        assert!(!aabb.is_point_inside(point));

        let other_aabb = Aabb {
            min: Vector3::new(-0.1, -0.1, -0.1),
            max: Vector3::new(0.1, 0.1, 0.1),
        };
        assert!(aabb.is_aabb_inside(&other_aabb));

        let other_aabb = Aabb {
            min: Vector3::new(99.9, -0.1, -0.1),
            max: Vector3::new(100.1, 0.1, 0.1),
        };
        assert!(!aabb.is_aabb_inside(&other_aabb));
    }
}