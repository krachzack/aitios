use super::Effect;

use ::geom::scene::Scene;
use ::geom::surf::{Surface, Surfel};

use std::path::Path;

use std::iter::IntoIterator;

/// Describes an aging effect concerning surface samples in isolation that can be described as a
/// linear equation such as:
///
///     substance1 = max(0.0, substance1 + rate * substance2)
///
/// For example, the accumulation of rust due to contact with water can be modelled as:
///
///     rust = rust + 0.2 * water
///
/// Assuming water is substance 0 and rust is substance 1, such an effect can be described in code
/// as `SurfelRule::new(0, 1, 0.2)`.
///
/// Similarly, the evaporation of water over time can be described as: `SurfelRule::new(0, 0, -0.5)`.
pub struct SurfelRule {
    write_substance_idx: usize,
    read_substance_idx: usize,
    rate: f32,
    applicable_materials: Vec<String>
}

impl SurfelRule {
    /// Creates a rule for the aging of surfels in isolotation. For example,
    /// water can evaportate over time, water can lead to more rust, etc.
    ///
    /// The last parameter limits the effect to the given material names. An empty thing that can be turned into
    /// an iterator indicates that the rule is applicable to all materials without exception, e.g.
    ///
    /// ```
    /// use std::iter::empty;
    ///
    /// // Material 0 should drop by 10% for all materials
    /// let drop_substance_zero = SurfelRule::new(0, 0, -0.1, empty());
    ///
    /// // Iron things should accumulate a substance 1 based on substance 0, 10% per iteration
    /// SurfelRule::new(0, 1, 0.1, [ "iron" ]);
    /// ```
    pub fn new<M, S>(write_substance_idx: usize, read_substance_idx: usize, rate: f32, applicable_materials: M) -> SurfelRule
        where M : IntoIterator<Item = S>, S : Into<String>
    {
        let applicable_materials = applicable_materials.into_iter().map(|m| m.into()).collect();
        SurfelRule { write_substance_idx, read_substance_idx, rate, applicable_materials }
    }

    fn perform_surfel(&self, surfel: &mut Surfel) {
        let &SurfelRule { write_substance_idx: write, read_substance_idx: read, rate, .. } = self;

        surfel.substances[write] = (surfel.substances[write] + rate * surfel.substances[read]).max(0.0);
    }

    fn applicable_material_idxs(&self, scene: &Scene) -> Vec<usize> {
        // Empty vector indicates for all materials
        if self.applicable_materials.is_empty() {
            return Vec::new()
        }

        // Non-empty vector can be translated into material indexes
        let applicable_idxs : Vec<usize> = scene.materials.iter()
            .enumerate()
            .filter(|&(_, scene_mat)| self.applicable_materials.iter()
                                          .any(|applicable_mat| applicable_mat == &scene_mat.name))
            .map(|(idx, _)| idx)
            .collect();

        assert!(
            applicable_idxs.len() > 0,
            "When non-empty target material names provided, at least one should actually exist. Target material names: {:?}, Scene materials: {:?}",
            self.applicable_materials,
            scene.materials
        );

        applicable_idxs
    }

    fn is_applicable(&self, mat_idx: usize, applicable_material_idxs: &Vec<usize>) -> bool {
        if applicable_material_idxs.is_empty() {
            true
        } else {
            applicable_material_idxs.iter()
                .any(|&idx| idx == mat_idx)
        }
    }
}

impl Effect for SurfelRule {
    fn perform(&self, scene: &mut Scene, surf: &mut Surface, _: &Path) {
        let applicable_material_idxs = self.applicable_material_idxs(scene);

        surf.samples.iter_mut()
            .filter(|s| self.is_applicable(scene.entities[s.entity_idx].material_idx, &applicable_material_idxs))
            .for_each(|s| self.perform_surfel(s))
    }
}
