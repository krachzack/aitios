//!
//! Contains functionality for triangles
//!

use ::cgmath::Vector3;
use ::cgmath::prelude::*;

use super::vtx::Vertex;
use super::spatial::Spatial;
use super::aabb::Aabb;

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

    /// Implements the [Möller–Trumbore intersection algorithm](https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm)
    /// for ray-triangle intersection. Note that this only intersects the front and not the back of the triangle
    pub fn ray_intersection_point(&self, ray_origin: Vector3<f32>, ray_direction: Vector3<f32>) -> Option<Vector3<f32>> {
        self.ray_intersection_parameter(ray_origin, ray_direction)
            .map(move |t| ray_origin + t * ray_direction)
    }

    pub fn ray_intersection_parameter(&self, ray_origin: Vector3<f32>, ray_direction: Vector3<f32>) -> Option<f32> {
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


pub fn intersect_ray_with_tri(ray_origin: &Vector3<f32>,
                              ray_direction: &Vector3<f32>,
                              vertex0: &Vector3<f32>,
                              vertex1: &Vector3<f32>,
                              vertex2: &Vector3<f32>) -> Option<Vector3<f32>> {

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

    let intersection_point = ray_origin + ray_direction * t;
    Some(intersection_point)
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
}

#[test]
fn test_intersect_ray_with_tri() {
    let ray_origin = Vector3::<f32>::new(0.0, 0.0, 0.0);
    let ray_direction = Vector3::<f32>::new(0.0, 0.0, 1.0);

    let vertex0 = Vector3::<f32>::new(-1.0, -1.0, 100.0);
    let vertex1 = Vector3::<f32>::new(1.0, -1.0, 100.0);
    let vertex2 = Vector3::<f32>::new(0.0, 1.0, 200.0);

    let intersection = intersect_ray_with_tri(
        &ray_origin, &ray_direction,
        &vertex0, &vertex1, &vertex2
    );

    assert_eq!(intersection, Some(Vector3::new(0.0, 0.0, 150.0)))
}