///! Contains the core particle tracing logic and invokes facilties
/// to emit gammatons, perform effects on the scene and to serialize
/// the scene in the end.

use std::fs;
use std::time::Instant;
use std::path::PathBuf;

use ::geom::surf::{Surface, Surfel, SurfaceBuilder};
use ::geom::scene::{Scene, Triangle};
use ::geom::octree::Octree;

use ::cgmath::Vector3;
use ::cgmath::prelude::*;

use ::sink::SceneSink;

use super::ton::{Ton, TonSource};
use super::effect::SceneEffect;

use ::rand;
use ::rand::Rng;

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

        info!("Tracing particles and transporting substances...  ");
        let before = Instant::now();
        let ton_to_surface_interaction_weight = self.ton_to_surface_interaction_weight;
        let surf = &mut self.surface;

        // First motion state is always trace straight
        self.sources.iter()
            .flat_map(|src| src.emit())
            .for_each(move |(mut ton, ray_origin, ray_direction)| Self::trace_straight(surf, &octree, &mut ton, ray_origin, ray_direction, ton_to_surface_interaction_weight));
        info!("Ok, took {}s", before.elapsed().as_secs());
    }

    fn trace_straight(surface: &mut Surface, octree: &Octree<Triangle>, ton: &mut Ton, origin: Vector3<f32>, direction: Vector3<f32>, ton_to_surface_interaction_weight: f32) {
        if let Some((_hit_tri_, param)) = octree.ray_intersection_target_and_parameter(origin, direction) {
            let intersection_point = origin + direction * param;
            let interacting_surfel_idxs = surface.find_within_sphere_indexes(intersection_point, ton.interaction_radius);

            if interacting_surfel_idxs.is_empty() {
                warn!("Ton intersected geometry but did not interact with any surfels, terminating early");
                return;
            }

            /*for surfel_idx in &interacting_surfel_idxs {
                let interacting_surfel = &mut surface.samples[*surfel_idx];

                // FIXME should only settled tons transport material? The fur simulation implements it that way
                Self::transport_material(ton, interacting_surfel, ton_to_surface_interaction_weight);
            }*/

            // TODO trace the particle further
            let mut rng = rand::thread_rng();
            let mut random : f32 = rng.gen();

            if random < (ton.p_straight + ton.p_parabolic + ton.p_flow) {
                // If not settled yet, pick up some material

                // REVIEW, should each interacting surfel deteriorate motion probabilities? Currently just one does
                Self::deteriorate_motion_probabilities(ton, &surface.samples[interacting_surfel_idxs[0]]);

                for surfel_idx in &interacting_surfel_idxs {
                    let interacting_surfel = &mut surface.samples[*surfel_idx];

                    // FIXME should only settled tons transport material? The fur simulation implements it that way
                    Self::transport_material_to_ton(interacting_surfel, ton, ton_to_surface_interaction_weight);
                }
            }

            if random < ton.p_straight {
                let random_on_unit_sphere = Vector3::new(
                    rng.next_f32(),
                    rng.next_f32(),
                    rng.next_f32()
                ).normalize();
                let normal = surface.samples[interacting_surfel_idxs[0]].normal;
                let outgoing_direction = (random_on_unit_sphere + normal).normalize();

                // TODO instead of taking the normal, sample on upper hemisphere, but I need tangents for this
                //let reflection_direction = normal;
                Self::trace_straight(surface, octree, ton, intersection_point + 0.000001 * normal, outgoing_direction, ton_to_surface_interaction_weight);
            } else if random < (ton.p_straight + ton.p_parabolic) {
                // TODO parabolic
            } else if random < (ton.p_straight + ton.p_parabolic + ton.p_flow) {
                // TODO flow
            } else {
                for surfel_idx in &interacting_surfel_idxs {
                    let interacting_surfel = &mut surface.samples[*surfel_idx];

                    // FIXME should only settled tons transport material? The fur simulation implements it that way
                    Self::transport_material_to_surf(ton, interacting_surfel, ton_to_surface_interaction_weight);
                }
            }
        }
    }

    fn transport_material_to_surf(ton: &Ton, interacting_surfel: &mut Surfel, ton_to_surface_interaction_weight: f32) {
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

    fn transport_material_to_ton(interacting_surfel: &mut Surfel, ton: &mut Ton, surface_to_ton_interaction_weight: f32) {
        assert_eq!(interacting_surfel.substances.len(), ton.substances.len());
        let material_transports = ton.substances
            .iter_mut()
            .zip(
                interacting_surfel.substances.iter_mut()
            );

        for (ref mut ton_material, ref mut surfel_material) in material_transports {
            let transport_amount = surface_to_ton_interaction_weight * **surfel_material;

            **surfel_material -= transport_amount;
            **ton_material += transport_amount;
        }
    }

    fn deteriorate_motion_probabilities(ton: &mut Ton, surfel: &Surfel) {
        ton.p_straight -= surfel.delta_straight;
        if ton.p_straight < 0.0 {
            ton.p_straight = 0.0;
        }

        ton.p_parabolic -= surfel.delta_parabolic;
        if ton.p_parabolic < 0.0 {
            ton.p_parabolic = 0.0;
        }

        // NOTE the original flow deterioration is max(kf + max(kp - deltaP, 0) - deltaF, 0)
        ton.p_flow -= surfel.delta_parabolic;
        if ton.p_flow < 0.0 {
            ton.p_flow = 0.0;
        }
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
