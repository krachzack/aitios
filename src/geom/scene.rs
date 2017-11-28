
use std::path::Path;
use ::tobj;
use ::geom::tri;
use ::cgmath::Vector3;
use ::cgmath::prelude::*;

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
                    // Use positions to calculate vertex count since normals and texcoords
                    // are optional and may be empty
                    let old_vtx_count = (scene.positions.len() / 3) as u32;
                    scene.indices.extend(
                        indices.iter().map(|i| old_vtx_count + i)
                    );
                    scene.positions.extend(positions);

                    // FIXME everything breaks down when some models have the optional
                    // fields and some have not. In this case, only positions are indexed properly
                    scene.normals.extend(normals);
                    scene.texcoords.extend(texcoords);

                    scene
                }
            )
    }

    /// Finds all intersections of the given ray with triangles in the scene.
    /// Front as well as back-facing triangles will be hit, the list of hits will not be sorted.
    /*pub fn intersect_all(&self, ray_origin: &Vector3<f32>, ray_direction: &Vector3<f32>) -> Vec<Vector3<f32>> {
        self.indices.chunks(3)
            .map(
                |i| (
                    Vector3::new(self.positions[(3*i[0]+0) as usize], self.positions[(3*i[0]+1) as usize], self.positions[(3*i[0]+2) as usize]),
                    Vector3::new(self.positions[(3*i[1]+0) as usize], self.positions[(3*i[1]+1) as usize], self.positions[(3*i[1]+2) as usize]),
                    Vector3::new(self.positions[(3*i[2]+0) as usize], self.positions[(3*i[2]+1) as usize], self.positions[(3*i[2]+2) as usize])
                )
            ).filter_map(
                |(v0, v1, v2)| tri::intersect_ray_with_tri(ray_origin, ray_direction, &v0, &v1, &v2)
            ).collect()
    }*/

    /// Finds the nearest intersection of the given ray with the triangles in the scene
    pub fn intersect(&self, ray_origin: &Vector3<f32>, ray_direction: &Vector3<f32>) -> Option<Vector3<f32>> {
        self.indices.chunks(3)
            .map(
                |i| (
                    Vector3::new(self.positions[(3*i[0]+0) as usize], self.positions[(3*i[0]+1) as usize], self.positions[(3*i[0]+2) as usize]),
                    Vector3::new(self.positions[(3*i[1]+0) as usize], self.positions[(3*i[1]+1) as usize], self.positions[(3*i[1]+2) as usize]),
                    Vector3::new(self.positions[(3*i[2]+0) as usize], self.positions[(3*i[2]+1) as usize], self.positions[(3*i[2]+2) as usize])
                )
            ).filter_map(
                |(v0, v1, v2)| tri::intersect_ray_with_tri(ray_origin, ray_direction, &v0, &v1, &v2)
            ).fold(
                None,
                |best_hit, hit| {
                    match best_hit {
                        None => Some(hit),
                        Some(best_hit) => {
                            if ray_origin.distance2(hit) < ray_origin.distance2(best_hit) { Some(hit) }
                            else { Some(best_hit) }
                        }
                    }
                }
            )
    }
}