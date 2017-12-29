use super::substance_map::SubstanceMap;

use ::geom::scene::Entity;

use ::tobj::Material;

/// Changes an entity using the information in the given associated substance map
pub trait SubstanceMapMaterialEffect {
    /// Optionally synthesizes a new material for the given entity with associated substance
    /// map and returns it.
    fn perform(&self, entity: &Entity, concentrations: &SubstanceMap) -> Option<Material>;
}
