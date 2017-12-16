
use super::Result;
use super::SceneSink;

use ::geom::scene::Scene;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

pub struct MtlSink {
    mtl_path: PathBuf
}

impl MtlSink {
    pub fn new(mtl_path: &str) -> MtlSink {
        MtlSink { mtl_path: PathBuf::from(mtl_path) }
    }
}

impl SceneSink for MtlSink {
    fn serialize(&self, scene: &Scene) -> Result<()> {
        let mut mtl = File::create(&self.mtl_path)?;

        info!("Writing MTL output file {:?}...", self.mtl_path);

        // Write header
        mtl.write("# aitios procedurally weathered MTL file\n".as_bytes())?;
        mtl.write(format!("# Material Count: {}\n", scene.materials.len()).as_bytes())?;

        for material in &scene.materials {
            mtl.write(format!("\nnewmtl {}\n", material.name).as_bytes())?;
            mtl.write(format!("Ns {}\n", material.shininess).as_bytes())?;
            mtl.write(format!("Ka {} {} {}\n", material.ambient[0], material.ambient[1], material.ambient[2]).as_bytes())?;
            mtl.write(format!("Kd {} {} {}\n", material.diffuse[0], material.diffuse[1], material.diffuse[2]).as_bytes())?;
            mtl.write(format!("Ks {} {} {}\n", material.specular[0], material.specular[1], material.specular[2]).as_bytes())?;
            mtl.write("Ke 0.000000 0.000000 0.000000\n".as_bytes())?;
            mtl.write("Ni 1.000000\n".as_bytes())?;
            mtl.write("d 1.000000\n".as_bytes())?;
            mtl.write(format!("illum {}\n", material.illumination_model.unwrap_or(1)).as_bytes())?;

            if !material.ambient_texture.is_empty() {
                mtl.write(format!("map_Ka {}\n", material.ambient_texture).as_bytes())?;
            }

            if !material.diffuse_texture.is_empty() {
                mtl.write(format!("map_Kd {}\n", material.diffuse_texture).as_bytes())?;
            }

            if !material.specular_texture.is_empty() {
                mtl.write(format!("map_Ks {}\n", material.specular_texture).as_bytes())?;
            }

            if !material.normal_texture.is_empty() {
                mtl.write(format!("norm {}\n", material.normal_texture).as_bytes())?;
            }
        }

        Ok(())
    }
}
