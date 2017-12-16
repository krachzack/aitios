
use std::fs;
use std::io::prelude::*;
use std::io;
use std::path::{Path, PathBuf};

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;

use ::sink::*;
use ::sink::obj::ObjSink;
use ::sink::mtl::MtlSink;

use super::sim::Simulation;
use super::ton::{TonSourceBuilder, TonSource};
use super::effect::{Effect, Blend, DensityMap};

pub struct SimulationBuilder {
    // TODO this should hold SceneBuilder and SurfaceBuilder
    scene: Scene,
    scene_directory: PathBuf,
    surface: Option<Surface>,
    iterations: u32,
    sources: Vec<TonSource>,
    effects: Vec<Box<Effect>>,
    scene_sinks: Vec<Box<SceneSink>>,
    ton_to_surface_interaction_weight: f32
}

impl SimulationBuilder {
    pub fn new() -> SimulationBuilder {
        SimulationBuilder {
            scene: Scene::empty(),
            scene_directory: PathBuf::new(),
            surface: None,
            iterations: 1,
            sources: Vec::new(),
            effects: Vec::new(),
            scene_sinks: Vec::new(),
            ton_to_surface_interaction_weight: 0.3
        }
    }

    pub fn scene<F>(mut self, scene_obj_file_path: &str, build_surface: F) -> SimulationBuilder
        where F: FnOnce(SurfaceBuilder) -> SurfaceBuilder
    {
        info!("Loading OBJ at {}... ", scene_obj_file_path);
        io::stdout().flush().unwrap();
        let scene = Scene::load_from_file(scene_obj_file_path);
        info!("Ok, {} triangles", scene.triangle_count());

        info!("Generating surface models from meshes... ");
        io::stdout().flush().unwrap();
        let surface = build_surface(SurfaceBuilder::new())
            .add_surface_from_scene(&scene)
            .build();
        info!("Ok, {} surfels", surface.samples.len());

        self.scene_directory = PathBuf::from(scene_obj_file_path);
        self.scene_directory.pop();
        self.scene = scene;
        self.surface = Some(surface);

        #[cfg(feature = "dump_surfels")]
        self.dump_surfels();

        self
    }

    #[cfg(feature = "dump_surfels")]
    fn dump_surfels(&self) {
        let surf_dump_file = "testdata/debug_surfels.obj";
        info!("Writing surface model to {}...", surf_dump_file);
        let mut surf_dump_file = fs::File::create(surf_dump_file).unwrap();
        match self.surface.as_ref().unwrap().dump(&mut surf_dump_file) {
            Ok(_) => info!("Ok"),
            Err(_) => info!("Failed")
        }
    }

    pub fn ton_to_surface_interaction_weight(mut self, ton_to_surface_interaction_weight: f32) -> SimulationBuilder {
        self.ton_to_surface_interaction_weight = ton_to_surface_interaction_weight;
        self
    }

    pub fn iterations(mut self, iterations: u32) -> SimulationBuilder {
        self.iterations = iterations;
        self
    }

    pub fn add_source<F>(mut self, build: F) -> SimulationBuilder
        where F: FnOnce(TonSourceBuilder) -> TonSourceBuilder {

        self.sources.push(
            build(TonSourceBuilder::new()).build()
        );

        self
    }

    pub fn add_effect_blend(mut self, substance_idx: usize, subject_material_name: &str, subject_material_map: &str, blend_towards_tex_file: &str, output_directory: &str) -> SimulationBuilder {
        self.effects.push(
            Box::new(
                Blend::new(substance_idx, &self.scene_directory, subject_material_name, subject_material_map, blend_towards_tex_file, &Path::new(output_directory))
            )
        );

        self
    }

    pub fn add_effect_density_map(mut self, map_width: usize, map_height: usize, output_directory: &str) -> SimulationBuilder {
        self.effects.push(
            Box::new(
                DensityMap::new(map_width, map_height, output_directory)
            )
        );

        self
    }

    pub fn add_scene_sink_obj_mtl(mut self, obj_file_path: &str, mtl_file_path: &str) -> SimulationBuilder {
        self.scene_sinks.push(Box::new(
            MtlSink::new(mtl_file_path)
        ));
        self.scene_sinks.push(Box::new(
            ObjSink::new(obj_file_path, Some(mtl_file_path))
        ));

        self
    }

    pub fn build(self) -> Simulation {
        Simulation::new(self.scene, self.surface.unwrap(), self.ton_to_surface_interaction_weight, self.iterations, self.sources, self.effects, self.scene_sinks)
    }
}
