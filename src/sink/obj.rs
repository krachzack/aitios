
use super::Result;
use super::SceneSink;

use ::geom::scene::Scene;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;

pub struct ObjSink {
    obj_path: PathBuf,
    mtl_lib: Option<String>
}

impl ObjSink {
    pub fn new(obj_path: &str, mtl_path: Option<&str>) -> ObjSink {
        let obj_path = PathBuf::from(obj_path);

        let mtl_lib = mtl_path.map(|s| {
            let mtl_path = PathBuf::from(s);
            let mtl_lib = mtl_path.file_name();

            assert_eq!(mtl_path.extension().unwrap().to_str().unwrap(), "mtl", "Expected an mtl path ending in .mtl, got {:?}", s);
            assert_eq!(mtl_path.parent(), obj_path.parent());

            String::from(mtl_lib.unwrap().to_str().unwrap())
        });

        assert!(obj_path.extension().is_some(), "Expected an obj path that ends with the extension .obj, got {:?}", obj_path);
        assert_eq!(obj_path.extension().unwrap().to_str().unwrap(), "obj", "Expected an obj path that ends with the extension .obj, got {:?}", obj_path);

        ObjSink { obj_path, mtl_lib }
    }
}

impl SceneSink for ObjSink {
    fn serialize(&self, scene: &Scene, output_prefix: &Path) -> Result<()> {
        let mut output_path = PathBuf::from(output_prefix);
        output_path.push(&self.obj_path);

        info!("Writing OBJ output file {:?}...", output_path);

        let mut obj = File::create(&output_path)?;

        // Write header
        obj.write("# aitios procedurally weathered OBJ file\n".as_bytes())?;
        if let Some(ref mtl_lib) = self.mtl_lib {
            obj.write("mtllib ".as_bytes())?;
            obj.write(mtl_lib.as_bytes())?;
            obj.write("\n".as_bytes())?;
        }
        obj.write("\n".as_bytes())?;

        let mut position_idx_base = 1_usize;
        let mut texcoord_idx_base = 1_usize;

        for entity in &scene.entities {
            obj.write("o ".as_bytes())?;
            obj.write(entity.name.as_bytes())?;
            obj.write("\n".as_bytes())?;

            let position_lines = entity.mesh.positions.chunks(3)
                .map(|p| format!("v {} {} {}\n", p[0], p[1], p[2]));

            for position_line in position_lines {
                obj.write(position_line.as_bytes())?;
            }

            let texcoord_lines = entity.mesh.texcoords.chunks(2)
                .map(|t| format!("vt {} {}\n", t[0], t[1]));

            for texcoord_line in texcoord_lines {
                obj.write(texcoord_line.as_bytes())?;
            }
            let material_idx = entity.material_idx;
            let material_name = &scene.materials[material_idx].name;
            obj.write(format!("usemtl {}\n", material_name).as_bytes())?;

            {
                let face_lines = entity.mesh.indices.chunks(3)
                    .map(|tri_indices| {
                        assert!(entity.mesh.texcoords.len() > 0);
                        format!(
                            "f {}/{} {}/{} {}/{}\n",
                            position_idx_base + (tri_indices[0] as usize), texcoord_idx_base + (tri_indices[0] as usize),
                            position_idx_base + (tri_indices[1] as usize), texcoord_idx_base + (tri_indices[1] as usize),
                            position_idx_base + (tri_indices[2] as usize), texcoord_idx_base + (tri_indices[2] as usize),
                        )
                    });

                for face_line in face_lines {
                    obj.write(face_line.as_bytes())?;
                }
            }

            obj.write("\n".as_bytes())?;

            position_idx_base += entity.mesh.positions.len() / 3;
            texcoord_idx_base += entity.mesh.texcoords.len() / 2;
        }

        Ok(())
    }
}
