use std::fs;
use std::io::prelude::*;

use std::io;
use std::time::Instant;

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;

use ::image;

use super::ton::TonSource;
use super::effect::Effect;

pub struct Simulation {
    scene: Scene,
    surface: Surface,
    iterations: u32,
    sources: Vec<TonSource>,
    effects: Vec<Box<Effect>>
}

impl Simulation {
    /// Creates a new simulation.
    /// Using the builder is recommended.
    pub fn new(scene: Scene, surface: Surface, iterations: u32, sources: Vec<TonSource>, effects: Vec<Box<Effect>>) -> Simulation {
        Simulation {
            scene,
            surface,
            iterations,
            sources,
            effects
        }
    }

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
