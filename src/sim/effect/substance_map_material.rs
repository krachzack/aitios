use super::substance_map::SubstanceMap;

use ::geom::scene::Entity;

use ::tobj::Material;

use std::path::Path;

/// Changes an entity using the information in the given associated substance map
pub trait SubstanceMapMaterialEffect {
    /// Optionally synthesizes a new material for the given entity with associated substance
    /// map and returns it. The prefix provides a base filename for synthesized files
    /// based on name and index of the entity, and index of the iteration
    fn perform(&self, entity: &Entity, concentrations: &SubstanceMap, prefix: &Path) -> Option<Material>;
}
