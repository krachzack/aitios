
use super::aabb::Aabb;

pub trait Spatial {
    /// Axis aligned bounding box of the spatial object
    fn bounds(&self) -> Aabb;
}

impl Spatial for Aabb {
    fn bounds(&self) -> Aabb {
        Aabb { ..*self }
    }
}