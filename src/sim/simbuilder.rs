
use std::fs;
use std::io::prelude::*;

use std::io;

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;

use super::sim::Simulation;
use super::ton::{TonSourceBuilder, TonSource};
use super::effect::{Effect, Blend};

pub struct SimulationBuilder {
    // TODO this should hold SceneBuilder and SurfaceBuilder
    scene: Scene,
    surface: Surface,
    iterations: u32,
    sources: Vec<TonSource>,
    effects: Vec<Box<Effect>>
}

impl SimulationBuilder {
    pub fn new() -> SimulationBuilder {
        SimulationBuilder {
            scene: Scene::empty(),
            surface: Surface { samples: Vec::new() },
            iterations: 1,
            sources: Vec::new(),
            effects: Vec::new()
        }
    }

    pub fn scene<F>(mut self, scene_obj_file_path: &str, build_surface: F) -> SimulationBuilder
    where F: FnOnce(SurfaceBuilder) -> SurfaceBuilder {
        print!("Loading OBJ at {}... ", scene_obj_file_path);
        io::stdout().flush().unwrap();
        let scene = Scene::load_from_file(scene_obj_file_path);
        println!("Ok, {} triangles", scene.triangle_count());

        print!("Generating surface models from meshes... ");
        io::stdout().flush().unwrap();
        let surface = build_surface(SurfaceBuilder::new())
            .add_surface_from_scene(&scene)
            .build();
        println!("Ok, {} surfels", surface.samples.len());

        self.scene = scene;
        self.surface = surface;

        #[cfg(feature = "dump_surfels")]
        self.dump_surfels();

        self
    }

    #[cfg(feature = "dump_surfels")]
    fn dump_surfels(&self) {
        let surf_dump_file = "testdata/debug_surfels.obj";
        print!("Writing surface model to {}... ", surf_dump_file);
        io::stdout().flush().unwrap();
        let mut surf_dump_file = fs::File::create(surf_dump_file).unwrap();
        match self.surface.dump(&mut surf_dump_file) {
            Ok(_) => println!("Ok"),
            Err(_) => println!("Failed")
        }
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

    pub fn add_effect_blend(mut self, substance_idx: usize, subject_material_name: &str, subject_material_map: &str, blend_towards_tex_file: &str) -> SimulationBuilder {
        self.effects.push(
            Box::new(
                Blend::new(substance_idx, subject_material_name, subject_material_map, blend_towards_tex_file)
            )
        );

        self
    }

    pub fn build(self) -> Simulation {
        Simulation::new(self.scene, self.surface, self.iterations, self.sources, self.effects)
    }
}
