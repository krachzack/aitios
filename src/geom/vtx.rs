
use ::cgmath::Vector3;

pub trait Vertex {
    fn position(&self) -> Vector3<f32>;
}
