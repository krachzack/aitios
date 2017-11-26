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