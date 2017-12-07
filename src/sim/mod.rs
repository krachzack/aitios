mod ton;
mod effect;

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;
use std::fs;
use std::io::prelude::*;
use std::io;
use std::time::{Instant};

use ::image;

use self::ton::{TonSourceBuilder, TonSource};
use self::effect::{Effect, Blend};

pub struct Simulation {
    scene: Scene,
    surface: Surface,
    iterations: u32,
    sources: Vec<TonSource>,
    effects: Vec<Box<Effect>>
}

pub struct SimulationBuilder {
    // TODO this should hold SceneBuilder and SurfaceBuilder
    scene: Scene,
    surface: Surface,
    iterations: u32,
    sources: Vec<TonSource>,
    effects: Vec<Box<Effect>>
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
                let ton_to_surface_interaction_weight = 0.15; // k value for accumulation of material
                **surfel_material = **surfel_material + ton_to_surface_interaction_weight * ton_material;
            }
        }
        println!("Ok, ton tracing took {} minutes", before.elapsed().as_secs() / 60);

        #[cfg(feature = "dump_hit_map")]
        self.dump_hit_map();

        #[cfg(feature = "dump_hit_texture")]
        self.dump_hit_texture();

        for effect in &self.effects {
            effect.perform(&self.scene, &self.surface)
        }
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

        let tex_width = 1024;
        let tex_height = 1024;

        // Generate one buffer and filename per entity
        let mut texes : Vec<_> = self.scene.entities.iter().enumerate()
            .map(|(idx, e)| {
                let filename = format!("testdata/{}-{}-hittex.png", idx, e.name);
                let tex_buf = image::ImageBuffer::from_fn(
                    tex_width, tex_height,
                    // Initialize with magenta so we see the texels that do not have a surfel nearby
                    |_, _| image::Rgb([255u8, 0u8, 255u8])
                );
                (filename, tex_buf)
            }).collect();

        // Draw surfels onto the textures
        for sample in &self.surface.samples {
            let (_, ref mut tex_buf) = texes[sample.entity_idx];

            let x = (sample.texcoords.x * (tex_width as f32)) as u32;
            // NOTE blender uses inversed v coordinate
            let y = ((1.0 - sample.texcoords.y) * (tex_height as f32)) as u32;

            if x >= tex_width || y >= tex_height {
                // Interpolation of texture coordinates can lead to degenerate uv coordinates
                // e.g. < 0 or > 1
                // In such cases, do not try to save the surfel but ingore it
                continue;
            }

            let intensity = (sample.substances[0] * 255.0) as u8;

            // TODO right now we overwrite when we find something brighter
            let overwrite = {
                let previous_intensity = tex_buf.get_pixel(x, y);
                previous_intensity.data[1] <= intensity
            };

            if overwrite {
                //println!("[{},{}] = {}", x, y, intensity);
                tex_buf.put_pixel(x, y, image::Rgb([intensity, intensity, intensity]));   
            }
        }

        println!("Ok");

        // Serialze them
        for (tex_file, tex_buf) in texes.into_iter() {
            print!("Writing {}... ", tex_file);
            io::stdout().flush().unwrap();

            let ref mut tex_file = fs::File::create(tex_file).unwrap();
            let _ = image::ImageRgb8(tex_buf).save(tex_file, image::PNG).unwrap();

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
        Simulation {
            scene: self.scene,
            surface: self.surface,
            iterations: self.iterations,
            sources: self.sources,
            effects: self.effects
        }
    }
}
