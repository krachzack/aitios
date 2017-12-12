//! Contains functionality to load scenes from OBJ files and to iterate
//!
//!

use std::path::Path;

use ::tobj;
use ::cgmath::{Vector2, Vector3};
use ::cgmath::prelude::*;

use super::tri;
use super::spatial::Spatial;
use super::aabb::Aabb;
use super::vtx;

pub type Triangle = tri::Triangle<Vertex>;

pub struct Scene {
    pub entities: Vec<Entity>,
    /// Materials as loaded from the OBJ, not the substances being carried
    pub materials: Vec<tobj::Material>
}

pub struct Entity {
    pub name: String,
    pub material_idx: usize,
    pub mesh: Mesh
}

pub struct Mesh {
    pub indices: Vec<u32>,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub texcoords: Vec<f32>
}

#[derive(Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub texcoords: Vector2<f32>,
    pub material_idx: usize,
    pub entity_idx: usize
}

impl vtx::Vertex for Vertex {
    fn position(&self) -> Vector3<f32> {
        self.position
    }
}

impl Scene {
    pub fn empty() -> Scene {
        Scene {
            entities: Vec::new(),
            materials: Vec::new()
        }
    }

    /// Loads the obj file at the given file system path into a newly created scene
    /// All contained models will be merged into a single mesh
    pub fn load_from_file(obj_file_path: &str) -> Scene {
        let (models, materials) = tobj::load_obj(&Path::new(obj_file_path)).unwrap();

        Scene {
            entities: models.into_iter()
                .map(move |m| {
                    Entity {
                        name: m.name,
                        material_idx: m.mesh.material_id.unwrap(), // All meshes need a material, otherwise panic
                        mesh: Mesh {
                            indices: m.mesh.indices,
                            positions: m.mesh.positions,
                            normals: m.mesh.normals,
                            texcoords: m.mesh.texcoords
                        }
                    }
                })
                .collect(),
            materials
        }
    }

    /// Returns an iterator over the triangles in all meshes
    pub fn triangles<'a>(&'a self) -> Box<Iterator<Item = Triangle> + 'a> {
        Box::new(
            self.entities.iter().enumerate()
                .flat_map(
                    |(entity_idx, e)| {
                        let mesh = &e.mesh;
                        let positions = &mesh.positions;
                        let texcoords = &mesh.texcoords;
                        let material_idx = e.material_idx;

                        mesh.indices.chunks(3)
                            .map(
                                move |i| Triangle::new(
                                    Vertex {
                                        position: Vector3::new(positions[(3*i[0]+0) as usize], positions[(3*i[0]+1) as usize], positions[(3*i[0]+2) as usize]),
                                        texcoords: Vector2::new(texcoords[(2*i[0]+0) as usize], texcoords[(2*i[0]+1) as usize]),
                                        material_idx,
                                        entity_idx
                                    },
                                    Vertex {
                                        position: Vector3::new(positions[(3*i[1]+0) as usize], positions[(3*i[1]+1) as usize], positions[(3*i[1]+2) as usize]),
                                        texcoords: Vector2::new(texcoords[(2*i[1]+0) as usize], texcoords[(2*i[1]+1) as usize]),
                                        material_idx,
                                        entity_idx
                                    },
                                    Vertex {
                                        position: Vector3::new(positions[(3*i[2]+0) as usize], positions[(3*i[2]+1) as usize], positions[(3*i[2]+2) as usize]),
                                        texcoords: Vector2::new(texcoords[(2*i[2]+0) as usize], texcoords[(2*i[2]+1) as usize]),
                                        material_idx,
                                        entity_idx
                                    }
                                )
                            )
                    }
                )
        )
    }

    /// Finds the nearest intersection of the given ray with the triangles in the scene
    pub fn intersect(&self, ray_origin: &Vector3<f32>, ray_direction: &Vector3<f32>) -> Option<Vector3<f32>> {
        self.entities.iter()
            .flat_map(
                |e| {
                    let mesh = &e.mesh;
                    let positions = &mesh.positions;

                    mesh.indices.chunks(3)
                        .map(
                            move |i| (
                                Vector3::new(positions[(3*i[0]+0) as usize], positions[(3*i[0]+1) as usize], positions[(3*i[0]+2) as usize]),
                                Vector3::new(positions[(3*i[1]+0) as usize], positions[(3*i[1]+1) as usize], positions[(3*i[1]+2) as usize]),
                                Vector3::new(positions[(3*i[2]+0) as usize], positions[(3*i[2]+1) as usize], positions[(3*i[2]+2) as usize])
                            )
                        )
                }
            )
            .filter_map(
                |(v0, v1, v2)| tri::intersect_ray_with_tri(ray_origin, ray_direction, &v0, &v1, &v2)
            )
            .min_by(|i0, i1|
                // We assume no NaN or infinities, that's why whe need to unwrap
                ray_origin.distance2(*i0).partial_cmp(&ray_origin.distance2(*i1)).unwrap()
            )
    }

    /// Calculates total triangle count in scene
    pub fn triangle_count(&self) -> usize {
        self.entities.iter().map(|e| e.mesh.indices.len() / 3).sum()
    }
}

/*impl Triangle {


    /// Gets the texture coordinates on the point that is closest to
    /// p on the triangle.
    pub fn texcoords_at(&self, p: Vector3<f32>) -> Vector2<f32>{
        let weights = self.barycentric_at(p);
        let coords = self.vertices.iter().map(|v| v.texcoords);

        weights.iter()
            .zip(coords)
            .map(|(w, c)| *w * c)
            .sum()
    }
}
*/