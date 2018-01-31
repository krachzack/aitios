//! Contains the core particle tracing logic and invokes facilties
//! to emit gammatons, perform effects on the scene and to serialize
//! the scene in the end.

use std::fs;
use std::time::Instant;
use std::path::PathBuf;
use std::f32::{INFINITY};

use ::geom::surf::{Surface, Surfel, SurfaceBuilder};
use ::geom::scene::{Scene, Triangle};
use ::geom::octree::Octree;
use ::geom::vtx::Position;
use ::geom::spatial::Spatial;

use ::cgmath::Vector3;
use ::cgmath::prelude::*;

use ::sink::SceneSink;

use super::ton::{Ton, TonSource};
use super::effect::Effect;

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
    effects: Vec<Box<Effect>>,
    /// Scene sinks that will be invoked after the completion of an iteration to serialize
    /// scene or materials.
    scene_sinks: Vec<Box<SceneSink>>,
    /// Base path for synthesized output files
    output_path: PathBuf,
    /// If set, holds the path where to write an obj with the subset of the surfels that were hit
    hit_map_path: Option<PathBuf>,
}

impl Simulation {
    /// Creates a new simulation.
    /// Using the builder is recommended.
    pub fn new(
        scene: Scene,
        surface: Surface,
        iterations: u32,
        sources: Vec<TonSource>,
        effects: Vec<Box<Effect>>,
        scene_sinks: Vec<Box<SceneSink>>,
        output_path: PathBuf,
        hit_map_path: Option<PathBuf>) -> Simulation
    {
        Simulation {
            scene,
            surface,
            iterations,
            sources,
            effects,
            scene_sinks,
            output_path,
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
            "Running simulation with {} iterations of {} particles each... ",
            self.iterations,
            self.sources.iter().map(|s| s.emission_count()).sum::<u32>()
        );

        for iteration_idx in 0..self.iterations {
            info!("Iteration {} started...", (1+iteration_idx));
            self.output_path.push(format!("iteration-{}", (1+iteration_idx)));
            fs::create_dir_all(&self.output_path).expect(&format!("Could not create iteration output directory {:?}", self.output_path));

            self.trace_particles();
            self.perform_iteration_effects();
            self.serialize_scene_to_sinks();

            self.output_path.pop();
        }

        self.dump_hit_map();
    }

    fn interact(surface: &mut Surface, octree: &Octree<Triangle>, ton: &mut Ton, hit_tri: &Triangle, intersection_point: Vector3<f32>, incoming_direction: Vector3<f32>) {
        let interacting_surfel_idxs = surface.find_within_sphere_indexes(intersection_point, ton.interaction_radius);

        if interacting_surfel_idxs.is_empty() {
            warn!("Ton intersected geometry but did not interact with any surfels, terminating early");
            return;
        }

        let mut rng = rand::thread_rng();
        let random : f32 = rng.gen();

        let &mut Ton { p_straight, p_parabolic, p_flow, .. } = ton;

        if random < (p_straight + p_parabolic + p_flow) {
            // If not settled yet, pick up some material

            // REVIEW, should each interacting surfel deteriorate motion probabilities? Currently just one does
            Self::deteriorate_motion_probabilities(ton, &surface.samples[interacting_surfel_idxs[0]]);
            Self::transport_material_to_ton(surface, &interacting_surfel_idxs, ton);
        }

        if random < p_straight {
            let normal = surface.samples[interacting_surfel_idxs[0]].normal;
            let outgoing_direction = hit_tri.sample_diffuse();

            // TODO instead of taking the normal, sample on upper hemisphere, but I need tangents for this
            //let reflection_direction = normal;
            Self::trace_straight(surface, octree, ton, intersection_point + 0.000001 * normal, outgoing_direction);
        } else if random < (p_straight + p_parabolic) {
            Self::trace_parabolic(surface, octree, ton, hit_tri, intersection_point);
        } else if random < (p_straight + p_parabolic + p_flow) {
            Self::trace_flow(surface, octree, ton, hit_tri, intersection_point, incoming_direction);
        } else {
            Self::transport_material_to_surf(ton, surface, &interacting_surfel_idxs);
            return;
        }
    }

    fn trace_particles(&mut self) {
        info!("Building octree...  ");
        let before = Instant::now();
        let octree : Octree<_> = self.scene.triangles().collect();
        info!("Done building octree after {}s", before.elapsed().as_secs());

        info!("Tracing particles and transporting substances...  ");
        let before = Instant::now();
        let surf = &mut self.surface;

        // First motion state is always trace straight
        self.sources.iter()
            .flat_map(|src| src.emit())
            .for_each(move |(mut ton, ray_origin, ray_direction)| Self::trace_straight(surf, &octree, &mut ton, ray_origin, ray_direction));
        info!("Ok, took {}s", before.elapsed().as_secs());
    }

    fn trace_straight(surface: &mut Surface, octree: &Octree<Triangle>, ton: &mut Ton, origin: Vector3<f32>, direction: Vector3<f32>) {
        if let Some((hit_tri, param)) = octree.ray_intersection_target_and_parameter(origin, direction) {
            let intersection_point = origin + direction * param;
            Self::interact(surface, octree, ton, hit_tri, intersection_point, direction);
        }
    }

    fn trace_flow(surface: &mut Surface, octree: &Octree<Triangle>, ton: &mut Ton,  hit_tri: &Triangle, intersection_point: Vector3<f32>, incoming_direction: Vector3<f32>) {
        let normal = hit_tri.normal();

        let origin_offset_mag = ton.flow_upward_offset; // both affect the distance of a flow event
        let downward_pull_mag = ton.flow_downward_pull;

        let new_origin = intersection_point + origin_offset_mag * normal;
        let flow_direction = {
            let dir = hit_tri.project_onto_tangential_plane(incoming_direction);
            if dir.is_zero() {
                warn!("Incoming direction for flow is orthogonal, using A edge as flow direction");
                (hit_tri.vertices[2].position() - hit_tri.vertices[1].position()).normalize()
            } else {
                dir
            }
        };
        let new_direction = (flow_direction - downward_pull_mag * normal).normalize();

        // TODO somehow handle misses, we dont wanna go 10 in X direction
        // maybe if misses, rescue by making it parabolic

        Self::trace_straight(surface, octree, ton, new_origin, new_direction);
    }

    fn trace_parabolic(surface: &mut Surface, octree: &Octree<Triangle>, ton: &mut Ton,  hit_tri: &Triangle, intersection_point: Vector3<f32>) {
        // Maximum height of a bounce assuming it is straight up and gravity pointing straight down
        let upward_parabola_height = ton.parabola_height;
        let gravity_mag = 9.81_f32;
        let timestep = 1.0 / 30.0; // 0.0333333 seconds, more is more exact but slower

        let gravity_acceleration = Vector3::new(0.0, -gravity_mag, 0.0);
        let takeoff_velocity_mag = (2.0 * gravity_mag * upward_parabola_height).sqrt();

        // REVIEW regarding surface as diffuse, could also reflect on the normal
        let normal = hit_tri.interpolate_at(intersection_point, |v| v.normal);
        let mut velocity = takeoff_velocity_mag * hit_tri.sample_diffuse();
        let mut position = intersection_point + normal * 0.0000001;
        let mut scene_bounds = octree.bounds();
        scene_bounds.max.y = INFINITY; // ignore if out of bounds in positive y direction since gravity will eventually pull it downward

        while scene_bounds.is_point_inside(position) {
            velocity += gravity_acceleration * timestep;

            let spatial_delta = velocity * timestep;
            let dist = spatial_delta.magnitude();
            let direction = spatial_delta / dist;

            if let Some((hit_tri, t)) = octree.line_segment_intersection_target_and_parameter(position, direction, dist) {
                let intersection_point = position + t * direction;
                Self::interact(surface, octree, ton, hit_tri, intersection_point, direction);
                break;
            } else {
                // No intersection, safe to move particle without penetrating objects
                position += spatial_delta;
            }
        }
    }

    fn transport_material_to_surf(ton: &Ton, surface: &mut Surface, interacting_surfel_idxs: &Vec<usize>) {
        for surfel_idx in interacting_surfel_idxs {
            let interacting_surfel = &mut surface.samples[*surfel_idx];
            assert_eq!(interacting_surfel.substances.len(), ton.substances.len());

            let material_transports = interacting_surfel.deposition_rates.iter()
                .zip(
                    interacting_surfel.substances
                        .iter_mut()
                        .zip(
                            ton.substances.iter()
                        )
                );

            for (ref deposition_rate, (ref mut surfel_material, &ton_material)) in material_transports {
                // Deposition rate gets equally divided between interacting surfels
                let deposition_rate = *deposition_rate / (interacting_surfel_idxs.len() as f32);
                **surfel_material = (**surfel_material + deposition_rate * ton_material).min(1.0);
            }
        }
    }

    fn transport_material_to_ton(surface: &mut Surface, interacting_surfel_idxs: &Vec<usize>, ton: &mut Ton) {
        for surfel_idx in interacting_surfel_idxs {
            let interacting_surfel = &mut surface.samples[*surfel_idx];

            assert_eq!(interacting_surfel.substances.len(), ton.substances.len());
            let material_transports = ton.pickup_rates.iter()
                .zip(
                    ton.substances
                        .iter_mut()
                        .zip(
                            interacting_surfel.substances.iter_mut()
                        )
                );

            for (ref pickup_rate, (ref mut ton_material, ref mut surfel_material)) in material_transports {
                // pickup rate gets equally distributed between all interacting surfels
                let pickup_rate = *pickup_rate / (interacting_surfel_idxs.len() as f32);
                let transport_amount = pickup_rate * **surfel_material;

                **surfel_material = (**surfel_material - transport_amount).max(0.0);
                **ton_material = (**ton_material + transport_amount).min(1.0);
            }
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
        ton.p_flow -= surfel.delta_flow;
        if ton.p_flow < 0.0 {
            ton.p_flow = 0.0;
        }
    }

    fn perform_iteration_effects(&mut self) {
        for effect in &self.effects {
            effect.perform(&mut self.scene, &mut self.surface, &self.output_path);
        }
    }

    fn serialize_scene_to_sinks(&self) {
        for sink in &self.scene_sinks {
            sink.serialize(&self.scene, &self.output_path).unwrap();
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
