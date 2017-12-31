//! Contains functionality to load scenes from OBJ files and to iterate
//!
//!

use std::path::Path;

use ::tobj;
use ::cgmath::{Vector2, Vector3};

use super::tri;
use super::vtx;
use super::intersect::IntersectRay;

pub type Triangle = tri::Triangle<Vertex>;

pub struct Scene {
    pub entities: Vec<Entity>,
    /// Materials as loaded from the OBJ, not the substances being carried
    pub materials: Vec<tobj::Material>
}

pub struct Entity {
    pub name: String,
    pub entity_idx: usize,
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
    pub normal: Vector3<f32>,
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
                .enumerate()
                .map(move |(idx, m)| {
                    Entity {
                        name: m.name,
                        entity_idx: idx,
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
                        let normals = &mesh.normals;
                        let texcoords = &mesh.texcoords;
                        let material_idx = e.material_idx;

                        assert!(!normals.is_empty());
                        assert!(!texcoords.is_empty());

                        mesh.indices.chunks(3)
                            .map(
                                move |i| Triangle::new(
                                    Vertex {
                                        position: Vector3::new(positions[(3*i[0]+0) as usize], positions[(3*i[0]+1) as usize], positions[(3*i[0]+2) as usize]),
                                        normal: Vector3::new(normals[(3*i[0]+0) as usize], normals[(3*i[0]+1) as usize], normals[(3*i[0]+2) as usize]),
                                        texcoords: Vector2::new(texcoords[(2*i[0]+0) as usize], texcoords[(2*i[0]+1) as usize]),
                                        material_idx,
                                        entity_idx
                                    },
                                    Vertex {
                                        position: Vector3::new(positions[(3*i[1]+0) as usize], positions[(3*i[1]+1) as usize], positions[(3*i[1]+2) as usize]),
                                        normal: Vector3::new(normals[(3*i[1]+0) as usize], normals[(3*i[1]+1) as usize], normals[(3*i[1]+2) as usize]),
                                        texcoords: Vector2::new(texcoords[(2*i[1]+0) as usize], texcoords[(2*i[1]+1) as usize]),
                                        material_idx,
                                        entity_idx
                                    },
                                    Vertex {
                                        position: Vector3::new(positions[(3*i[2]+0) as usize], positions[(3*i[2]+1) as usize], positions[(3*i[2]+2) as usize]),
                                        normal: Vector3::new(normals[(3*i[2]+0) as usize], normals[(3*i[2]+1) as usize], normals[(3*i[2]+2) as usize]),
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
    pub fn intersect(&self, ray_origin: Vector3<f32>, ray_direction: Vector3<f32>) -> Option<Vector3<f32>> {
        // find lowest t in ray(t) = ray_origin + t * ray_direction
        let t = self.triangles()
            .filter_map(
                |t| t.ray_intersection_parameter(ray_origin, ray_direction)
            )
            .min_by(|t0, t1| t0.partial_cmp(&t1).unwrap());

        t.map(|t| ray_origin + t * ray_direction)
    }

    /// Calculates total triangle count in scene
    pub fn triangle_count(&self) -> usize {
        self.entities.iter().map(|e| e.mesh.indices.len() / 3).sum()
    }
}

impl Entity {
    pub fn triangles<'a>(&'a self) -> Box<Iterator<Item = Triangle> + 'a> {
        let material_idx = self.material_idx;
        let entity_idx = self.entity_idx;

        let mesh = &self.mesh;
        let positions = &mesh.positions;
        let normals = &mesh.normals;
        let texcoords = &mesh.texcoords;

        assert!(!normals.is_empty());
        assert!(!texcoords.is_empty());

        Box::new(
            mesh.indices.chunks(3)
                .map(
                    move |i| Triangle::new(
                        Vertex {
                            position: Vector3::new(positions[(3*i[0]+0) as usize], positions[(3*i[0]+1) as usize], positions[(3*i[0]+2) as usize]),
                            normal: Vector3::new(normals[(3*i[0]+0) as usize], normals[(3*i[0]+1) as usize], normals[(3*i[0]+2) as usize]),
                            texcoords: Vector2::new(texcoords[(2*i[0]+0) as usize], texcoords[(2*i[0]+1) as usize]),
                            material_idx,
                            entity_idx
                        },
                        Vertex {
                            position: Vector3::new(positions[(3*i[1]+0) as usize], positions[(3*i[1]+1) as usize], positions[(3*i[1]+2) as usize]),
                            normal: Vector3::new(normals[(3*i[1]+0) as usize], normals[(3*i[1]+1) as usize], normals[(3*i[1]+2) as usize]),
                            texcoords: Vector2::new(texcoords[(2*i[1]+0) as usize], texcoords[(2*i[1]+1) as usize]),
                            material_idx,
                            entity_idx
                        },
                        Vertex {
                            position: Vector3::new(positions[(3*i[2]+0) as usize], positions[(3*i[2]+1) as usize], positions[(3*i[2]+2) as usize]),
                            normal: Vector3::new(normals[(3*i[2]+0) as usize], normals[(3*i[2]+1) as usize], normals[(3*i[2]+2) as usize]),
                            texcoords: Vector2::new(texcoords[(2*i[2]+0) as usize], texcoords[(2*i[2]+1) as usize]),
                            material_idx,
                            entity_idx
                        }
                    )
                )
        )
    }
}
