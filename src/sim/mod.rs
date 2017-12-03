mod ton;

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;
use std::fs;
use std::io::prelude::*;
use std::io;
use std::time::{Instant};

use ::cgmath::Vector3;

use ::image;

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
        print!("Finding initial intersections...  ");
        io::stdout().flush().unwrap();
        let before = Instant::now();
        let initial_hits : Vec<_> = self.sources.iter()
            .flat_map(|src| src.emit())
            .filter_map(|(ton, ray_origin, ray_direction)|
                self.scene.intersect(&ray_origin, &ray_direction).map(|p| (ton, p) )
            ).collect();
        println!("Ok, took {} minutes", before.elapsed().as_secs() / 60);

        println!("Starting ton tracing...");
        let before = Instant::now();
        let mut iteration_nr = 1;
        let print_progress_interval = if initial_hits.len() < 10 { 1 } else { initial_hits.len() / 10 };
        for &(ref ton, intersection_point) in initial_hits.iter() {
            let interacting_surfel = self.surface.nearest(intersection_point);

            if (iteration_nr % print_progress_interval) == 0 {
                println!("Tracing materials... {}%", (100.0 * (iteration_nr as f64) / (initial_hits.len() as f64)).round());
            }

            iteration_nr += 1;

            assert_eq!(interacting_surfel.substances.len(), ton.substances.len());
            let material_transports = interacting_surfel.substances
                .iter_mut()
                .zip(
                    ton.substances.iter()
                );

            for (ref mut surfel_material, &ton_material) in material_transports {
                let ton_to_surface_interaction_weight = 0.06; // k value for accumulation of material
                **surfel_material = **surfel_material + ton_to_surface_interaction_weight * ton_material;
            }
        }
        println!("Ok, ton tracing took {} minutes", before.elapsed().as_secs() / 60);

        #[cfg(feature = "dump_hit_map")]
        self.dump_hit_map();

        #[cfg(feature = "dump_hit_texture")]
        self.dump_hit_texture();
    }

    #[cfg(feature = "dump_hit_map")]
    fn dump_hit_map(&self) {
        let hit_map_file = "testdata/debug_hits.obj";
        print!("Dumping interacted surfels to {}... ", hit_map_file);
        io::stdout().flush().unwrap();

        let hit_map = self.surface.samples.iter()
        .filter_map(|s| if s.substances[0] > 0.0 { Some(s.position) } else { None });

        let hit_map = SurfaceBuilder::new()
            .add_surface_from_points(hit_map)
            .build();

        hit_map.dump(&mut fs::File::create(hit_map_file).unwrap()).unwrap();

        println!("Ok");
    }

    #[cfg(feature = "dump_hit_texture")]
    fn dump_hit_texture(&self) {
        print!("Collecting interacted surfels into textures... ");
        io::stdout().flush().unwrap();

        let tex_width = 128;
        let tex_height = 128;

        // Generate one buffer and filename per source material
        let mut texes : Vec<_> = self.scene.materials.iter()
            .map(|m| {
                let filename = format!("testdata/{}-hittex.png", m.diffuse_texture);
                let tex_buf = image::ImageBuffer::new(tex_width, tex_height);
                (filename, tex_buf)
            }).collect();

        // Draw surfels onto the textures
        for sample in &self.surface.samples {
            let (_, ref mut tex_buf) = texes[sample.material_idx];

            let x = (sample.texcoords.x * (tex_width as f32)) as u32;
            // NOTE blender uses inversed v coordinate
            let y = ((1.0 - sample.texcoords.y) * (tex_height as f32)) as u32;
            let intensity = (sample.substances[0] * 255.0) as u8;

            // TODO right now we overwrite when we find something brighter
            let overwrite = {
                let previous_intensity : &image::Luma<u8> = tex_buf.get_pixel(x, y);
                previous_intensity.data[0] < intensity
            };

            if overwrite {
                //println!("[{},{}] = {}", x, y, intensity);
                tex_buf.put_pixel(x, y, image::Luma([intensity]));   
            }
        }

        println!("Ok");

        // Serialze them
        for (tex_file, tex_buf) in texes.into_iter() {
            print!("Writing {}... ", tex_file);
            io::stdout().flush().unwrap();

            let ref mut tex_file = fs::File::create(tex_file).unwrap();
            let _ = image::ImageLuma8(tex_buf).save(tex_file, image::PNG).unwrap();

            println!("Ok");
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
        println!("Ok, {} triangles", scene.triangle_count());

        print!("Generating surface models from meshes... ");
        io::stdout().flush().unwrap();
        let surface = SurfaceBuilder::new()
                        .delta_straight(1.0)
                        // just one substance on the surfels with an initial value of 0.0 for all surfels
                        .substances(vec![0.0])
                        .add_surface_from_scene(&scene, 3000.0)
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
                    .substances(&vec![1.0])
                    .point_shaped(&position)
                    .emission_count(30000)
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
