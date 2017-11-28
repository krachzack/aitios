mod ton;

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;
use std::fs;
use std::io::prelude::*;
use std::io;

use ::cgmath::Vector3;

use self::ton::{Ton, TonSource, TonSourceBuilder};

struct Simulation {
    scene: Scene,
    surface: Surface
}

impl Simulation {
    fn new(scene_obj_file_path: &str) -> Simulation {
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

        Simulation { scene, surface }
    }

    fn iterate(&self, source_position: Vector3<f32>, particle_count: u32) {
        print!("Starting simulation iteration with {} particles... ", particle_count);
        io::stdout().flush().unwrap();

        let source = TonSourceBuilder::new()
            .p_straight(1.0)
            .materials(&vec![1.0])
            .point_shaped(&source_position)
            .build();


        let particle_hits = source.emit(particle_count)
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

pub fn simulate(scene_obj_file_path: &str) {
    let sim = Simulation::new(scene_obj_file_path);
    sim.iterate(Vector3::new(0.0, 5.0, 0.05), 5000);
}
