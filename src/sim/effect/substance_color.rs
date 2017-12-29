
use super::substance_map::SubstanceMap;
use super::substance_map_material::SubstanceMapMaterialEffect;

use ::geom::scene::Entity;

use ::cgmath::Vector4;

use ::tobj::Material;

pub struct SubstanceColorEffect {
    zero_color: Vector4<f32>,
    one_color: Vector4<f32>,
    unused_color: Vector4<f32>
}

impl SubstanceMapMaterialEffect for SubstanceColorEffect {
    fn perform(&self, entity: &Entity, concentrations: &SubstanceMap) -> Option<Material> {
        None
    }
}
