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
        println!("Running simulation with {} particles... ", self.sources.iter().map(|s| s.emission_count()).sum::<u32>());
        for _ in 0..self.iterations {
            self.iterate()
        }
    }

    fn iterate(&mut self) {
        println!("Finding initial intersections...");

        let initial_hits : Vec<_> = self.sources.iter()
            .flat_map(|src| src.emit())
            .filter_map(|(ton, ray_origin, ray_direction)|
                self.scene.intersect(&ray_origin, &ray_direction).map(|p| (ton, p) )
            ).collect();

        println!("Tracing...");
        let mut iteration_nr = 1;
        for &(ref ton, intersection_point) in initial_hits.iter() {
            let interacting_surfel = self.surface.nearest(intersection_point);

            println!("Interacting ton {} of {}", iteration_nr, initial_hits.len());
            iteration_nr += 1;

            assert_eq!(interacting_surfel.materials.len(), ton.materials.len());
            let material_transports = interacting_surfel.materials
                .iter_mut()
                .zip(
                    ton.materials.iter()
                );

            for (ref mut surfel_material, &ton_material) in material_transports {
                let ton_to_surface_interaction_weight = 0.05; // k value for accumulation of material
                **surfel_material = **surfel_material + ton_to_surface_interaction_weight * ton_material;
            }
        }

        #[cfg(feature = "dump_hit_map")]
        self.dump_hit_map();
    }

    #[cfg(feature = "dump_hit_map")]
    fn dump_hit_map(&self) {
        let hit_map_file = "testdata/debug_hits.obj";
        print!("Dumping interacted surfels to {}... ", hit_map_file);
        io::stdout().flush().unwrap();

        let hit_map = self.surface.samples.iter()
        .filter_map(|s| if s.materials[0] > 0.0 { Some(s.position) } else { None });

        let hit_map = SurfaceBuilder::new()
            .add_surface_from_points(hit_map)
            .build();

        hit_map.dump(&mut fs::File::create(hit_map_file).unwrap()).unwrap();

        println!("Ok");
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
        println!("Ok, {} triangles", scene.triangle_count());

        print!("Generating surface models from meshes... ");
        io::stdout().flush().unwrap();
        let surface = SurfaceBuilder::new()
                        .delta_straight(1.0)
                        // just one material on the surfels with an initial value of 0.0 for all surfels
                        .materials(vec![0.0])
                        .add_surface_from_scene(&scene, 2500.0)
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

    pub fn add_point_source(mut self, position: &Vector3<f32>) -> SimulationBuilder {
        self.sources.push(
            TonSourceBuilder::new()
                    .p_straight(1.0)
                    .materials(&vec![1.0])
                    .point_shaped(&position)
                    .emission_count(5000)
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
