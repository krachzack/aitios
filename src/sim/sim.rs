///! Contains the core particle tracing logic and invokes facilties
/// to emit gammatons, perform effects on the scene and to serialize
/// the scene in the end.

use std::fs;
use std::time::Instant;
use std::path::PathBuf;

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;
use ::geom::octree::Octree;
use ::geom::intersect::IntersectRay;

use ::sink::SceneSink;

use super::ton::TonSource;
use super::effect::SceneEffect;

/// Maintains a simulation on a scene with an associated surface
/// model.
pub struct Simulation {
    /// The scene that is owned by the scene and will be modified when running the simulation
    scene: Scene,
    /// The surface model, describing surface properties at point samples of the scene
    surface: Surface,
    /// Amount of iterations to perform, each involving the tracing of newly emitted particles and
    /// the performing of effects.
    iterations: u32,
    /// Ton sources that will emit particles at the start of each iteration
    sources: Vec<TonSource>,
    /// Effects that will be invoked at the end of each iteration
    scene_effects: Vec<Box<SceneEffect>>,
    /// Scene sinks that will be invoked after the completion of the last iteration to serialize
    /// scene or materials.
    scene_sinks: Vec<Box<SceneSink>>,
    /// Determines how much of a substance stored in a ton will be transferred to an interacted
    /// surfel.
    ///
    /// The general form is:
    /// surfel.substance[n] = surfel.substance[n] + ton_to_surface_interaction_weight * ton.substance[n]
    ton_to_surface_interaction_weight: f32,
    /// If set, holds the path where to write an obj with the subset of the surfels that were hit
    hit_map_path: Option<PathBuf>,
}

impl Simulation {
    /// Creates a new simulation.
    /// Using the builder is recommended.
    pub fn new(
        scene: Scene,
        surface: Surface,
        ton_to_surface_interaction_weight: f32,
        iterations: u32,
        sources: Vec<TonSource>,
        scene_effects: Vec<Box<SceneEffect>>,
        scene_sinks: Vec<Box<SceneSink>>,
        hit_map_path: Option<PathBuf>) -> Simulation
    {
        Simulation {
            scene,
            surface,
            iterations,
            sources,
            scene_effects,
            scene_sinks,
            ton_to_surface_interaction_weight,
            hit_map_path
        }
    }

    /// Runs the simulation for the set amount of iterations. Each iteration
    /// involves:
    ///
    /// * mutating the surface with particle tracing,
    /// * applying effects to the scene with information from the mutated surface.
    ///
    /// After tracing is complete, the scene sinks will be invoked to serialize the
    /// modified scene and materials.
    pub fn run(&mut self) {
        info!(
            "Running simulation with {} iteraions of {} particles each... ",
            self.iterations,
            self.sources.iter().map(|s| s.emission_count()).sum::<u32>()
        );

        for _ in 0..self.iterations {
            self.trace_particles();
            self.perform_iteration_effects();
        }

        self.perform_after_simulation_effects();

        self.serialize_scene_to_sinks();
        self.dump_hit_map();
    }

    fn trace_particles(&mut self) {
        info!("Building octree...  ");
        let before = Instant::now();
        let octree : Octree<_> = self.scene.triangles().collect();
        info!("Done building octree after {}s", before.elapsed().as_secs());

        info!("Finding initial intersections...  ");
        let before = Instant::now();
        let initial_hits : Vec<_> = self.sources.iter()
            .flat_map(|src| src.emit())
            .filter_map(|(ton, ray_origin, ray_direction)|
                octree.ray_intersection_point(ray_origin, ray_direction)
                    .map(|p| (ton, p) )
            ).collect();
        info!("Ok, took {}s", before.elapsed().as_secs());

        info!("Starting ton tracing... ");
        let before = Instant::now();
        let ton_to_surface_interaction_weight = self.ton_to_surface_interaction_weight;
        for &(ref ton, intersection_point) in initial_hits.iter() {
            let interacting_surfel_idxs = self.surface.find_within_sphere_indexes(intersection_point, ton.interaction_radius);

            for surfel_idx in interacting_surfel_idxs {
                let interacting_surfel = &mut self.surface.samples[surfel_idx];

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
        }
        info!("Ok, {}s", before.elapsed().as_secs());
    }

    fn perform_iteration_effects(&mut self) {
        for effect in &self.scene_effects {
            effect.perform_after_iteration(&mut self.scene, &self.surface);
        }
    }

    fn perform_after_simulation_effects(&mut self) {
        for effect in &self.scene_effects {
            effect.perform_after_simulation(&mut self.scene, &self.surface);
        }
    }

    fn serialize_scene_to_sinks(&self) {
        for sink in &self.scene_sinks {
            sink.serialize(&self.scene).unwrap();
        }
    }

    fn dump_hit_map(&self) {
        if let Some(hit_map_path) = self.hit_map_path.as_ref() {
            info!("Dumping interacted surfels to {:?}... ", hit_map_path);

            let hit_map = self.surface.samples.iter()
                .filter_map(|s| if s.substances[0] > 0.0 { Some(s.position) } else { None });

            let hit_map = SurfaceBuilder::new()
                .add_surface_from_points(hit_map)
                .build();

            hit_map.dump(&mut fs::File::create(hit_map_path).unwrap()).unwrap();

            info!("Ok");
        }
    }
}
