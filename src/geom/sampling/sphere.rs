
use ::geom::tri::Triangle;
use ::geom::vtx::Position;
use ::cgmath::Vector3;
use ::cgmath::prelude::*;
use ::rand::{self, Rng};

pub fn uniform_on_unit_sphere() -> Vector3<f32> {
    let mut rng = rand::thread_rng();

    Vector3::new(
        rng.next_f32() - 0.5,
        rng.next_f32() - 0.5,
        rng.next_f32() - 0.5
    ).normalize()
}

// TODO not uniform, instead try: http://holger.dammertz.org/stuff/notes_HammersleyOnHemisphere.html
pub fn uniform_on_unit_z_hemisphere() -> Vector3<f32> {
    let mut rng = rand::thread_rng();

     Vector3::new(
        rng.next_f32() - 0.5,
        rng.next_f32() - 0.5,
        rng.next_f32() * 0.5
    ).normalize()
}
