use ::geom::tri::Triangle;
use ::geom::vtx::Vertex;

use ::cgmath::Vector3;

use std::ops::{Mul, Add};

use ::kdtree::kdtree::KdtreePointTrait;

/// Vertex consisting of position and the index of a triangle that
/// this vertex originated from. By having a small vertex type, we can
/// more cheaply create new triangles and interpolate vertices.
#[derive(Copy, Clone)]
pub struct SparseVertex {
    pub mother_triangle_idx: Option<usize>,
    pub position: [f64; 3]
}

impl PartialEq for SparseVertex {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

impl Vertex for SparseVertex {
    fn position(&self) -> Vector3<f32> {
        Vector3::new(self.position[0] as f32, self.position[1] as f32, self.position[2] as f32)
    }
}

impl Mul<f32> for SparseVertex {
    type Output = SparseVertex;

    fn mul(self, scalar: f32) -> Self::Output {
        SparseVertex {
            mother_triangle_idx: self.mother_triangle_idx,
            position: [
                self.position[0] * (scalar as f64),
                self.position[1] * (scalar as f64),
                self.position[2] * (scalar as f64)
            ]
        }
    }
}

impl Add for SparseVertex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        // Mother riangle is taken from lhs, but is assumed
        // to by identical for rhs
        SparseVertex {
            mother_triangle_idx: self.mother_triangle_idx,
            position: [
                self.position[0] + rhs.position[0],
                self.position[1] + rhs.position[1],
                self.position[2] + rhs.position[2]
            ]
        }
    }
}

impl KdtreePointTrait for SparseVertex {
    #[inline]
    fn dims(&self) -> &[f64] {
        &self.position
    }
}
