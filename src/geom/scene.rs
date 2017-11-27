
use ::tobj;
use std::path::Path;

pub struct Scene {
    pub indices: Vec<u32>,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub texcoords: Vec<f32>
}

impl Scene {
    /// Loads the obj file at the given file system path into a newly created scene
    /// All contained models will be merged into a single mesh
    pub fn load_from_file(obj_file_path: &str) -> Scene {
        let (models, _materials) = tobj::load_obj(&Path::new(obj_file_path)).unwrap();

        models.iter()
            .map(|m| (&m.mesh.indices, &m.mesh.positions, &m.mesh.normals, &m.mesh.texcoords))
            // Accumulate models into a single scene object
            .fold(
                Scene { indices: Vec::new(), positions: Vec::new(), normals: Vec::new(), texcoords: Vec::new() },
                move |mut scene, (indices, positions, normals, texcoords)| {
                    let old_vtx_count = (scene.positions.len() / 3) as u32;
                    scene.indices.extend(
                        indices.iter().map(|i| old_vtx_count + i)
                    );
                    scene.positions.extend(positions);
                    scene.normals.extend(normals);
                    scene.texcoords.extend(texcoords);

                    scene
                }
            )
    }
}