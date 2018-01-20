//!
//! Contains functionality for triangles.
//!

use ::cgmath::{Vector3, Matrix3};
use ::cgmath::prelude::*;

use super::vtx::{Position, Normal};
use super::spatial::Spatial;
use super::aabb::Aabb;
use super::intersect::IntersectRay;

use std::ops::{Mul, Add};
use std::iter::Sum;
use std::f32::EPSILON;

use ::rand;

/// The `Triangle<V>` type encapsulates three vertices.
/// A vertex must implement `geom::vtx::Vertex` and hence has a position
/// in 3D space.
#[derive(Debug, Copy, Clone)]
pub struct Triangle<V>
    where V : Position
{
    pub vertices: [V; 3]
}

impl<V> Spatial for Triangle<V>
    where V : Position
{
    fn bounds(&self) -> Aabb {
        Aabb::from_points(
            self.vertices.iter()
                .map(|v| v.position())
        )
    }
}

impl<V> Triangle<V>
    where V : Position
{
    pub fn new(vertex0: V, vertex1: V, vertex2: V) -> Triangle<V> {
        Triangle {
            vertices: [vertex0, vertex1, vertex2]
        }
    }

    /// ||(V1 −V0)×(V2 −V0)||/2
    pub fn area(&self) -> f32 {
        let v0 = self.vertices[0].position();
        let v1 = self.vertices[1].position();
        let v2 = self.vertices[2].position();

        0.5 * ((v1 - v0).cross(v2 - v0)).magnitude()
    }

    /// Calculates the area of the triangle specified with the three vertices
    /// using Heron's formula
    /*pub fn area(&self) -> f32 {
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
    }*/

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

        // this is the vector from a vertex A to the circumsphere center
        let to_circumsphere_center = (ab_cross_ac.cross(ab) * ac.magnitude2() + ac.cross(ab_cross_ac) * ab.magnitude2()) /
            (2.0 * ab_cross_ac.magnitude2());

        a +  to_circumsphere_center
    }

    /// Returns the minimum bounding sphere center and squared radius
    /// of the triangle.
    ///
    /// See: http://realtimecollisiondetection.net/blog/?p=20
    pub fn minimum_bounding_sphere_sqr(&self) -> (Vector3<f32>, f32) {
        //void MinimumBoundingCircle(Circle &circle, Point a, Point b, Point c) {
        let (a, b, c) = (self.vertices[0].position(), self.vertices[1].position(), self.vertices[2].position());
        let dot_abab = (b - a).dot(b - a);
        let dot_abac = (b - a).dot(c - a);
        let dot_acac = (c - a).dot(c - a);
        let d = 2.0 * (dot_abab * dot_acac - dot_abac * dot_abac);
        let mut reference_point = a;

        let center = if d.abs() <= EPSILON {
            // a, b, and c lie on a line. Circle center is center of AABB of the
            // points, and radius is distance from circle center to AABB corner
            let bbox = self.bounds();
            reference_point = bbox.min;
            0.5 * (bbox.min + bbox.max)
        } else {
            let s = (dot_abab * dot_acac - dot_acac * dot_abac) / d;
            let t = (dot_acac * dot_abab - dot_abab * dot_abac) / d;
            // s controls height over AC, t over AB, (1-s-t) over BC
            if s <= 0.0 {
                0.5 * (a + c)
            } else if t <= 0.0 {
                0.5 * (a + b)
            } else if (s + t) >= 1.0 {
                reference_point = b;
                0.5 * (b + c)
            } else {
                a + s*(b - a) + t*(c - a)
            }
        };

        let radius_sqr = center.distance2(reference_point);

        (center, radius_sqr)
    }

    pub fn minimum_bounding_sphere_center(&self) -> Vector3<f32> {
        let (center, _) = self.minimum_bounding_sphere();
        center
    }

    pub fn minimum_bounding_sphere(&self) -> (Vector3<f32>, f32) {
        let (center, radius_sqr) = self.minimum_bounding_sphere_sqr();
        (center, radius_sqr.sqrt())
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

    pub fn interpolate_bary<F, T>(&self, weights: [f32; 3], vertex_to_val_fn: F) -> T
        where F: Fn(&V) -> T,
            T: Sum<<T as Mul<f32>>::Output> + Mul<f32>
    {
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

    /// Calculates a tangent space based on vertex positions and returns it as three
    /// vectors that form an orthonormal basis. The first vector will be a tangent,
    /// the second a binormal and the third the face normal.
    ///
    /// Note that there are infinite possible tangent spaces. The resulting tangent
    /// is parallel to the edge C, that is from vertex 0 to vertex 1. The
    /// tangent space is not guaranteed to be aligned with texture space, which is
    /// normally a common way to align it.
    pub fn tangent_space(&self) -> (Vector3<f32>, Vector3<f32>, Vector3<f32>) {
        let (a, b, c) = (
            self.vertices[0].position(),
            self.vertices[1].position(),
            self.vertices[2].position()
        );

        let a_to_b = b - a;
        let a_to_c = c - a;

        let scaled_normal = a_to_b.cross(a_to_c);

        // If two points are colinear, result is always zero vector: v.cross(v) = 0, v.cross(Vector3::zero()) = 0
        assert!(!scaled_normal.is_zero(), "Face normal is undefined for triangle with zero area: [{:?}, {:?}, {:?}]", a, b, c);

        let normal = scaled_normal.normalize();
        let tangent = a_to_b.normalize();
        let binormal = (normal.cross(tangent)).normalize();

        (tangent, binormal, normal)
    }

    pub fn tangent(&self) -> Vector3<f32> {
        let (tangent, _, _) = self.tangent_space();
        tangent
    }

    pub fn binormal(&self) -> Vector3<f32> {
        let (_, binormal, _) = self.tangent_space();
        binormal
    }

    /// Calculates a face normal for the triangle based on the vertex positions
    /// and the cross product.
    ///
    /// Panics for empty triangles, since the resulting cross product is always zero.
    pub fn normal(&self) -> Vector3<f32> {
        let (_, _, normal) = self.tangent_space();
        normal
    }

    pub fn world_to_tangent_matrix(&self) -> Matrix3<f32> {
        let (tangent, binormal, normal) = self.tangent_space();
        Matrix3::from_cols(tangent, binormal, normal).transpose()
    }

    /// Transforms the given direction vector into tangent space, setting the height component to zero and then
    /// transforming back into world space. The resulting direction should be parallel to the tangential plane
    /// in world space. If the given direction happens to be parallel to the normal, a zero vector is returned.
    pub fn project_onto_tangential_plane(&self, incoming_direction_world: Vector3<f32>) -> Vector3<f32> {
        let world_to_tangent = self.world_to_tangent_matrix();
        let tangent_to_world = world_to_tangent.invert()
            .expect("Expected tangent space matrix to be invertible");

        let tangent_space_direction = world_to_tangent * incoming_direction_world;
        let tangent_space_direction_flat = tangent_space_direction
            .truncate() // drop Z
            .extend(0.0); // and set to zero

        let scaled_projected = tangent_to_world * tangent_space_direction_flat;

        if scaled_projected.is_zero() {
            scaled_projected
        } else {
            scaled_projected.normalize()
        }
    }
}

impl<V> Triangle<V>
    where V : Position + Normal
{
    /// Synthesizes a face normal by averaging the normals of the vertices
    pub fn face_normal_by_vertices(&self) -> Vector3<f32> {
        (1.0 / 3.0) * self.vertices.iter()
            .map(|v| v.normal())
            .fold(Vector3::zero(), |acc, n| acc + n)
    }
}

impl<V> Triangle<V>
    where V : Position + Clone + Mul<f32, Output = V> + Add<V, Output = V>
{
    pub fn sample_position(&self) -> Vector3<f32> {
        let positions = self.vertices.iter().map(|v| v.position());
        random_bary().iter()
            .zip(positions)
            .map(|(&bary, vtx)| bary * vtx)
            .fold(Vector3::zero(), |acc, vtx| acc + vtx)
    }

    /// Interpolates a vertex on a random position on the triangle
    pub fn sample_vertex(&self) -> V {
        self.interpolate_vertex_at_bary(random_bary())
    }

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
    where V : Position
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

pub fn random_bary() -> [f32; 3] {
    let u = rand::random::<f32>();
    let v = rand::random::<f32>();

    let sqrt_u = u.sqrt();

    [
        1.0 - sqrt_u,
        (sqrt_u * (1.0 - v)),
        (sqrt_u * v)
    ]
}

#[cfg(test)]
mod test {
    use super::*;

    struct Vtx(Vector3<f32>);

    impl Position for Vtx {
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

        let subdivided_tris_area_sum = triangles.iter().map(Triangle::area).sum::<f32>();
        let source_tri_area = tri.area();
        assert_eq!(source_tri_area, 2.0); // (width * height) / 2, given width = 2, height = 2
        assert_eq!(subdivided_tris_area_sum, source_tri_area);
    }

    #[test]
    fn test_calculate_face_normal_from_positions() {
        // ccw on X/Z-Plane, normal should point in positive Y direction
        let tri = Triangle::new(
            Vector3::new(-1.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, -1.0)
        );

        let (tangent, binormal, normal) = tri.tangent_space();
        assert_eq!(Vector3::new(0.0, 1.0, 0.0), normal);
        assert_eq!(Vector3::new(1.0, 0.0, 0.0), tangent);
        assert_eq!(Vector3::new(0.0, 0.0, -1.0), binormal);

        // cw, normal should point down
        let tri = Triangle::new(
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(-1.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, -1.0)
        );

        assert_eq!(Vector3::new(0.0, -1.0, 0.0), tri.normal());
    }

    #[test]
    #[should_panic]
    fn test_zero_area_triangle_normal_calculation_panics() {
        let tri = Triangle::new(
            Vector3::new(-1.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0)
        );

        tri.normal();
    }

    #[test]
    fn test_project_direction_on_tangential_plane_on_floor() {
        // triangle flat on the floor with normal facing toward y
        // projecting should just drop the Z value in this case
        let floor_tri = Triangle::new(
            Vector3::new(-1.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, -1.0)
        );

        let up_right_positive_z = Vector3::new(1.0, 1.0, 1.0).normalize();

        assert_eq!(
            Vector3::new(1.0, 0.0, 1.0).normalize(),
            floor_tri.project_onto_tangential_plane(up_right_positive_z),
            "Projecting a vector onto a triangle flat on the floor should yield the same vector with the z value dropped"
        );
    }

    #[test]
    fn test_project_direction_on_slope_tri() {
        // Triangle with 45° upward slope in X direction
        let slope_tri = Triangle::new(
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, -1.0)
        );

        let down = Vector3::new(0.0, -1.0, 0.0);
        let projected = slope_tri.project_onto_tangential_plane(down);

        assert_eq!(
            Vector3::new(-1.0, -1.0, 0.0).normalize(),
            projected
        );
    }

    #[test]
    fn test_project_direction_parallel_to_normal() {
        // triangle flat on the floor with normal facing toward y
        // projecting should just drop the Z value in this case
        let floor_tri = Triangle::new(
            Vector3::new(-1.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, -1.0)
        );

        let up = Vector3::new(0.0, 1.0, 0.0).normalize();

        assert_eq!(
            Vector3::new(0.0, 0.0, 0.0),
            floor_tri.project_onto_tangential_plane(up),
            "Projecting a vector onto a triangle flat on the floor should yield the same vector with the z value dropped"
        );
    }
}
