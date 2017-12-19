//!
//! Contains functionality for triangles.
//!

use ::cgmath::Vector3;
use ::cgmath::prelude::*;

use super::vtx::Vertex;
use super::spatial::Spatial;
use super::aabb::Aabb;
use super::intersect::IntersectRay;

use std::ops::{Mul, Add};
use std::iter::Sum;

/// The `Triangle<V>` type encapsulates three vertices.
/// A vertex must implement `geom::vtx::Vertex` and hence has a position
/// in 3D space.
#[derive(Copy, Clone)]
pub struct Triangle<V>
    where V : Vertex
{
    pub vertices: [V; 3]
}

impl<V> Spatial for Triangle<V>
    where V : Vertex
{
    fn bounds(&self) -> Aabb {
        Aabb::from_points(
            self.vertices.iter()
                .map(|v| v.position())
        )
    }
}

impl<V> Triangle<V>
    where V : Vertex
{
    pub fn new(vertex0: V, vertex1: V, vertex2: V) -> Triangle<V> {
        Triangle {
            vertices: [vertex0, vertex1, vertex2]
        }
    }

    /// Calculates the area of the triangle specified with the three vertices
    /// using Heron's formula
    pub fn area(&self) -> f32 {
        let p0 = self.vertices[0].position();
        let p1 = self.vertices[1].position();
        let p2 = self.vertices[2].position();

        // calculate sidelength
        let a = (p0 - p1).magnitude();
        let b = (p1 - p2).magnitude();
        let c = (p2 - p0).magnitude();

        // s is halved circumference
        let s = (a + b + c) / 2.0;

        (s * (s - a) * (s - b) * (s - c)).sqrt()
    }

    pub fn center(&self) -> Vector3<f32> {
        let one_over_three =  1.0 / 3.0;
        self.vertices.iter()
            .map(|v| one_over_three * v.position())
            .sum()
    }

    /// Gets the center of a sphere that runs through all of the three
    /// triangle vertices.
    pub fn circumcenter(&self) -> Vector3<f32> {
        let (a, b, c) = (self.vertices[0].position(), self.vertices[1].position(), self.vertices[2].position());

        let ac = c - a;
        let ab = b - a;
        let ab_cross_ac = ab.cross(ac);

        // this is the vector from a TO the circumsphere center
        let to_circumsphere_center = (ab_cross_ac.cross(ab) * ac.magnitude2() + ac.cross( ab_cross_ac )*ab.magnitude2()) /
            (2.0 * ab_cross_ac.magnitude2());

        a +  to_circumsphere_center
    }

    /// Compute barycentric coordinates [u, v, w] for
    /// the closest point to p on the triangle.
    pub fn barycentric_at(&self, p: Vector3<f32>) -> [f32; 3] {
        let v0 = self.vertices[1].position() - self.vertices[0].position();
        let v1 = self.vertices[2].position() - self.vertices[0].position();
        let v2 = p - self.vertices[0].position();

        let d00 = v0.dot(v0);
        let d01 = v0.dot(v1);
        let d11 = v1.dot(v1);
        let d20 = v2.dot(v0);
        let d21 = v2.dot(v1);
        let denom = d00 * d11 - d01 * d01;

        let v = (d11 * d20 - d01 * d21) / denom;
        let w = (d00 * d21 - d01 * d20) / denom;
        let u = 1.0 - v - w;

        [u, v, w]
    }

    pub fn interpolate_at<F, T>(&self, position: Vector3<f32>, vertex_to_val_fn: F) -> T
        where F: Fn(&V) -> T,
            T: Sum<<T as Mul<f32>>::Output> + Mul<f32>
    {
        let weights = self.barycentric_at(position);
        let values = self.vertices.iter().map(vertex_to_val_fn);

        weights.iter()
            .zip(values)
            .map(|(w, v)| v * *w)
            .sum()
    }

    /// Checks if the triangle is completely inside the given sphere
    pub fn is_inside_sphere(&self, center: Vector3<f32>, radius: f32) -> bool {
        let radius_sqr = radius * radius;
        self.vertices.iter()
            .all(|v| center.distance2(v.position()) < radius_sqr)
    }
}

impl<V> Triangle<V>
    where V : Vertex + Clone + Mul<f32, Output = V> + Add<V, Output = V>
{
    /// Synthesizes a new vertex at the given position.
    /// The position is converted to barycentric coordinates and
    /// the vertices blended together
    pub fn interpolate_vertex_at_position(&self, position: Vector3<f32>) -> V {
        self.interpolate_vertex_at_bary(self.barycentric_at(position))
    }

    /// Synthesizes a new vertex at the given position.
    /// The position is converted to barycentric coordinates and
    /// the vertices blended together
    pub fn interpolate_vertex_at_bary(&self, weights: [f32; 3]) -> V {
        let vertices = self.vertices.iter();

        let mut weighted_vertices = weights.iter()
            .zip(vertices)
            .map(|(w, v)| v.clone() * *w);

        weighted_vertices.next().unwrap() +
        weighted_vertices.next().unwrap() +
        weighted_vertices.next().unwrap()
    }

    pub fn split_at_edge_midpoints(&self) -> [Triangle<V>; 4] {
        let mids : [V; 3] = [
            self.interpolate_vertex_at_bary([0.5, 0.5, 0.0]),
            self.interpolate_vertex_at_bary([0.0, 0.5, 0.5]),
            self.interpolate_vertex_at_bary([0.5, 0.0, 0.5])
        ];

        let verts = &self.vertices;
        let (outer_tri0, outer_tri1, outer_tri2) = {
            let mut outer_triangles = (0..mids.len()).map(|mid_idx0| {
                let vert0_mid = mids[mid_idx0].clone();
                let vert1_vert = verts[(mid_idx0 + 1) % 3].clone();
                let vert2_mid = mids[(mid_idx0 + 1) % 3].clone();

                Triangle::new(vert0_mid, vert1_vert, vert2_mid)
            });

            (
                outer_triangles.next().unwrap(),
                outer_triangles.next().unwrap(),
                outer_triangles.next().unwrap()
            )
        };

        let inner_triangle = Triangle { vertices: mids };

        [
            inner_triangle,
            outer_tri0,
            outer_tri1,
            outer_tri2
        ]
    }
}

impl<V> IntersectRay for Triangle<V>
    where V : Vertex
{
    fn ray_intersection_parameter(&self, ray_origin: Vector3<f32>, ray_direction: Vector3<f32>) -> Option<f32> {
        let vertex0 = self.vertices[0].position();
        let vertex1 = self.vertices[1].position();
        let vertex2 = self.vertices[2].position();

        let epsilon = 0.0000001;

        let edge1 = vertex1 - vertex0;
        let edge2 = vertex2 - vertex0;

        let h = ray_direction.cross(edge2);
        let a = edge1.dot(h);

        if a > -epsilon && a < epsilon {
            return None;
        }

        let f = 1.0 / a;
        let s = ray_origin - vertex0;
        let u = f * (s.dot(h));

        if u < 0.0 || u > 1.0 {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * ray_direction.dot(q);

        if v < 0.0 || (u + v) > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);

        if t < epsilon {
            return None;
        }

        Some(t)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct Vtx(Vector3<f32>);

    impl Vertex for Vtx {
        fn position(&self) -> Vector3<f32> {
            self.0
        }
    }

    #[test]
    fn intersect_ray_with_tri() {
        let ray_origin = Vector3::<f32>::new(0.0, 0.0, 0.0);
        let ray_direction = Vector3::<f32>::new(0.0, 0.0, 1.0);

        let vertex0 = Vtx(Vector3::new(-1.0, -1.0, 100.0));
        let vertex1 = Vtx(Vector3::new(1.0, -1.0, 100.0));
        let vertex2 = Vtx(Vector3::new(0.0, 1.0, 200.0));

        let tri = Triangle::new(vertex0, vertex1, vertex2);

        assert!(tri.ray_intersection_parameter(ray_origin, ray_direction).unwrap() > 0.0);
        assert_eq!(tri.ray_intersection_point(ray_origin, ray_direction), Some(Vector3::new(0.0, 0.0, 150.0)));
    }

    #[test]
    fn intersect_ray_with_tri_and_miss() {
        let ray_origin = Vector3::<f32>::new(0.0, 0.0, 0.0);
        let ray_direction = Vector3::<f32>::new(0.0, 0.0, -1.0);

        let vertex0 = Vtx(Vector3::new(-1.0, -1.0, 100.0));
        let vertex1 = Vtx(Vector3::new(1.0, -1.0, 100.0));
        let vertex2 = Vtx(Vector3::new(0.0, 1.0, 200.0));

        let tri = Triangle::new(vertex0, vertex1, vertex2);

        assert_eq!(tri.ray_intersection_parameter(ray_origin, ray_direction), None);
        assert_eq!(tri.ray_intersection_point(ray_origin, ray_direction), None);
    }

    #[test]
    fn interpolate_position() {
        let vertex0 = Vtx(Vector3::new(-1.0, -1.0, 0.0));
        let vertex1 = Vtx(Vector3::new(1.0, -1.0, 0.0));
        let vertex2 = Vtx(Vector3::new(0.0, 1.0, 0.0));
        let tri = Triangle::new(vertex0, vertex1, vertex2);

        let point_on_there = Vector3::new(0.0, 0.5, 0.0);

        assert_eq!(
            point_on_there,
            tri.interpolate_at(point_on_there, |v| v.position()),
            "Interpolating the position value should yield the same point"
        );
    }

    #[test]
    fn test_splitting_at_edge_midpoints() {
        // A triangle around the origin
        let tri = Triangle::new(
            Vector3::new(-1.0, -1.0, 0.0),
            Vector3::new(1.0, -1.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0)
        );

        let triangles = tri.split_at_edge_midpoints();
        assert_eq!(4, triangles.len());

        // Three triangles should contain each edge midpoint as a vertex
        assert_eq!(
            3,
            triangles.iter()
                .filter(|t| {
                    t.vertices.iter()
                        .any(|v|
                            v.position().x == -0.5 &&
                            v.position().y == 0.0
                        )

                })
                .count()
        );

        assert_eq!(
            3,
            triangles.iter()
                .filter(|t| {
                    t.vertices.iter()
                        .any(|v|
                            v.position().x == 0.5 &&
                            v.position().y == 0.0
                        )

                })
                .count()
        );

        assert_eq!(
            3,
            triangles.iter()
                .filter(|t| {
                    t.vertices.iter()
                        .any(|v|
                            v.position().x == 0.0 &&
                            v.position().y == -1.0
                        )

                })
                .count()
        );
    }
}
