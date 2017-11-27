///
/// Contains functions for triangles
///

use ::cgmath::Vector3;
use ::cgmath::prelude::*;

/// Calculates the area of the triangle specified with the three vertices
/// using Heron's formula
pub fn area(p0: Vector3<f32>, p1: Vector3<f32>, p2: Vector3<f32>) -> f32 {
    // calculate sidelength
    let a = (p0 - p1).magnitude();
    let b = (p1 - p2).magnitude();
    let c = (p2 - p0).magnitude();

    // s is halved circumference
    let s = (a + b + c) / 2.0;

    (s * (s - a) * (s - b) * (s - c)).sqrt()
}

/// Implements the [Möller–Trumbore intersection algorithm](https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm)
/// for ray-triangle intersection. Note that this only intersects the front and not the back of the triangle
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