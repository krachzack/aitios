mod ton;

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;
use std::fs;
use std::io::prelude::*;
use std::io;

use ::cgmath::Vector3;

use self::ton::{TonSourceBuilder, TonSource};

pub struct Simulation {
    scene: Scene,
    surface: Surface,
    iterations: u32,
    sources: Vec<TonSource>
}

pub struct SimulationBuilder {
    // TODO this should hold SceneBuilder and SurfaceBuilder
    scene: Scene,
    surface: Surface,
    iterations: u32,
    sources: Vec<TonSource>
}

impl Simulation {
    pub fn run(&mut self) {
        print!("Running simulation... ");
        io::stdout().flush().unwrap();
        for _ in 0..self.iterations {
            self.iterate()
        }
    }

    fn iterate(&mut self) {
        let particle_hits = self.sources.iter()
            .flat_map(|src| src.emit())
            .filter_map(|(_, ray_origin, ray_direction)| self.scene.intersect(&ray_origin, &ray_direction));

        let hit_surface = SurfaceBuilder::new().add_surface_from_points(particle_hits).build();
        println!("Ok, {} hits", hit_surface.samples.len());

        print!("Writing hits to testdata/hits.obj... ");
        io::stdout().flush().unwrap();
        let mut surf_dump_file = fs::File::create("testdata/hits.obj").unwrap();
        match hit_surface.dump(&mut surf_dump_file) {
            Ok(_) => println!("Ok"),
            Err(_) => println!("Failed")
        }
    }
}

impl SimulationBuilder {
    pub fn new() -> SimulationBuilder {
        SimulationBuilder {
            scene: Scene::empty(),
            surface: Surface { samples: Vec::new() },
            iterations: 1,
            sources: Vec::new()
        }
    }

    pub fn scene(mut self, scene_obj_file_path: &str) -> SimulationBuilder {
        print!("Loading OBJ at {}... ", scene_obj_file_path);
        io::stdout().flush().unwrap();
        let scene = Scene::load_from_file(scene_obj_file_path);
        println!("Ok, {} triangles", scene.indices.len() / 3);

        print!("Generating surface models from meshes... ");
        io::stdout().flush().unwrap();
        let surface = SurfaceBuilder::new()
                        .delta_straight(1.0)
                        .add_surface_from_indexed_triangles(&scene.positions, &scene.indices)
                        // just one material on the surfels with an initial value of 0.0 for all surfels
                        .materials(vec![0.0])
                        .build();
        println!("Ok, {} surfels", surface.samples.len());

        let surf_dump_file = format!("{}.surfels.obj", scene_obj_file_path);
        print!("Writing surface model to {}... ", surf_dump_file);
        io::stdout().flush().unwrap();
        let mut surf_dump_file = fs::File::create(surf_dump_file).unwrap();
        match surface.dump(&mut surf_dump_file) {
            Ok(_) => println!("Ok"),
            Err(_) => println!("Failed")
        }

        self.scene = scene;
        self.surface = surface;

        self
    }

    pub fn iterations(mut self, iterations: u32) -> SimulationBuilder {
        self.iterations = iterations;
        self
    }

    pub fn add_point_source(mut self, position: &Vector3<f32>) -> SimulationBuilder {
        self.sources.push(
            TonSourceBuilder::new()
                    .p_straight(1.0)
                    .materials(&vec![1.0])
                    .point_shaped(&position)
                    .emission_count(10000)
                    .build()
        );
        self
    }

    pub fn build(self) -> Simulation {
        Simulation {
            scene: self.scene,
            surface: self.surface,
            iterations: self.iterations,
            sources: self.sources
        }
    }
}
