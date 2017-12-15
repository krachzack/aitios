use std::fs;
use std::io::prelude::*;

use std::io;
use std::time::Instant;

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;
use ::geom::octree::Octree;
use ::geom::intersect::IntersectRay;

use super::ton::TonSource;
use super::effect::Effect;

pub struct Simulation {
    scene: Scene,
    surface: Surface,
    iterations: u32,
    sources: Vec<TonSource>,
    effects: Vec<Box<Effect>>,
    /// Determines how much of a substance stored in a ton will be transferred to an interacted
    /// surfel.
    ///
    /// The general form is:
    /// surfel.substance[n] = surfel.substance[n] + ton_to_surface_interaction_weight * ton.substance[n]
    ton_to_surface_interaction_weight: f32
}

impl Simulation {
    /// Creates a new simulation.
    /// Using the builder is recommended.
    pub fn new(scene: Scene, surface: Surface, ton_to_surface_interaction_weight: f32, iterations: u32, sources: Vec<TonSource>, effects: Vec<Box<Effect>>) -> Simulation {
        Simulation {
            scene,
            surface,
            iterations,
            sources,
            effects,
            ton_to_surface_interaction_weight
        }
    }

    pub fn run(&mut self) {
        println!("Running simulation with {} particles... ", self.sources.iter().map(|s| s.emission_count()).sum::<u32>());
        for _ in 0..self.iterations {
            self.iterate()
        }
    }

    fn iterate(&mut self) {
        print!("Building octree...  ");
        io::stdout().flush().unwrap();
        let before = Instant::now();
        let octree : Octree<_> = self.scene.triangles().collect();
        println!("Ok, took {}s", before.elapsed().as_secs());

        print!("Finding initial intersections...  ");
        io::stdout().flush().unwrap();
        let before = Instant::now();
        let initial_hits : Vec<_> = self.sources.iter()
            .flat_map(|src| src.emit())
            .filter_map(|(ton, ray_origin, ray_direction)|
                octree.ray_intersection_point(ray_origin, ray_direction)
                    .map(|p| (ton, p) )
            ).collect();
        println!("Ok, took {}s", before.elapsed().as_secs());

        print!("Starting ton tracing...  ");
        io::stdout().flush().unwrap();
        let before = Instant::now();
        let ton_to_surface_interaction_weight = self.ton_to_surface_interaction_weight;
        for &(ref ton, intersection_point) in initial_hits.iter() {
            let interacting_surfel = self.surface.nearest(intersection_point);

            assert_eq!(interacting_surfel.substances.len(), ton.substances.len());
            let material_transports = interacting_surfel.substances
                .iter_mut()
                .zip(
                    ton.substances.iter()
                );

            for (ref mut surfel_material, &ton_material) in material_transports {
                **surfel_material = **surfel_material + ton_to_surface_interaction_weight * ton_material;
            }
        }
        println!("Ok, {}s", before.elapsed().as_secs());

        #[cfg(feature = "dump_hit_map")]
        self.dump_hit_map();

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
}
