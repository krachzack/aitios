
use ::cgmath::Vector3;

pub trait Vertex {
    fn position(&self) -> Vector3<f32>;
}

impl Vertex for Vector3<f32> {
    fn position(&self) -> Vector3<f32> {
        *self
    }
}

