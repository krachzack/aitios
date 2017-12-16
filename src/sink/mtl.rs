
use super::Result;
use super::SceneSink;

use ::geom::scene::Scene;

use std::path::PathBuf;

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
        Ok(())
    }
}
