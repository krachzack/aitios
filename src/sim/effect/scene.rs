
use ::geom::scene::Scene;
use ::geom::surf::Surface;

use std::path::Path;

/// Represents a weathering effect.
/// The effect can be applied after each iteration and/or after
/// the whole simulation.
pub trait SceneEffect {
    /// Applies an iterative weathering effect by mutating the referenced scene.
    /// Changed geometry will effect future iterations.
    fn perform_after_iteration(&self, scene: &mut Scene, surf: &Surface, output_prefix: &Path);

    /// Applies a weathering effect after all iterations have finished
    fn perform_after_simulation(&self, scene: &mut Scene, surf: &Surface);
}
