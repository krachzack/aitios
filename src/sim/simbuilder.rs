
use std::fs;
use std::io::prelude::*;
use std::io;
use std::path::{Path, PathBuf};

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;

use ::sink::*;
use ::sink::obj::ObjSink;
use ::sink::mtl::MtlSink;

use super::sim::Simulation;
use super::ton::{TonSourceBuilder, TonSource};
use super::effect::{Effect, Blend, DensityMap};

/// Builds a simulation according to provided parameters and closures.
///
/// # Examples
///
/// The following example builds, but not runs, a simulation on
/// `"test-scenes/buddha-scene/buddha-scene.obj"` where the diffuse map
/// of the `"bronze"` material is blended towards an image file
/// `"test-scenes/buddha-scene/weathered_bronze.png"`.
///
/// ```
/// # #[cfg_attr(not(feature = "expensive_tests"), ignore)]
/// use aitios::SimulationBuilder;
///
/// SimulationBuilder::new()
///     .scene(
///         "test-scenes/buddha-scene/buddha-scene.obj",
///         |s| {
///             s.surfels_per_sqr_unit(5000.0)
///                 .delta_straight(1.0)
///                 .substances(&vec![0.0])
///         }
///     )
///     .add_source(|s| {
///         s.p_straight(1.0)
///             .substances(&vec![1.0])
///             .point_shaped(0.0, 2.0, 0.0)
///             .emission_count(40000)
///     })
///     .add_effect_blend(
///         0, // Index of substance that drives the blend
///         "bronze", // material that gets changed
///         "map_Kd", // map of the material that gets changed
///         "test-scenes/buddha-scene/weathered_bronze.png",
///         "output/weathered_bronze.png"
///     )
///     .add_scene_sink_obj_mtl(
///         "output/buddha-scene-weathered.obj",
///         "output/buddha-scene-weathered.mtl"
///     )
///     .ton_to_surface_interaction_weight(0.05)
///     .iterations(1)
///     .build();
/// ```
pub struct SimulationBuilder {
    // TODO this should hold SceneBuilder and SurfaceBuilder
    scene: Scene,
    scene_directory: PathBuf,
    surface: Option<Surface>,
    iterations: u32,
    sources: Vec<TonSource>,
    effects: Vec<Box<Effect>>,
    scene_sinks: Vec<Box<SceneSink>>,
    ton_to_surface_interaction_weight: f32,
    hit_map_path: Option<PathBuf>,
    surfel_obj_path: Option<PathBuf>
}

impl SimulationBuilder {
    pub fn new() -> SimulationBuilder {
        SimulationBuilder {
            scene: Scene::empty(),
            scene_directory: PathBuf::new(),
            surface: None,
            iterations: 1,
            sources: Vec::new(),
            effects: Vec::new(),
            scene_sinks: Vec::new(),
            ton_to_surface_interaction_weight: 0.3,
            hit_map_path: None,
            surfel_obj_path: None
        }
    }

    pub fn scene<F>(mut self, scene_obj_file_path: &str, build_surface: F) -> SimulationBuilder
        where F: FnOnce(SurfaceBuilder) -> SurfaceBuilder
    {
        info!("Loading OBJ at {}... ", scene_obj_file_path);
        io::stdout().flush().unwrap();
        let scene = Scene::load_from_file(scene_obj_file_path);
        info!("Ok, {} triangles", scene.triangle_count());

        debug!("look: {}", scene.materials[0].diffuse_texture);

        info!("Generating surface models from meshes... ");
        io::stdout().flush().unwrap();
        let surface = build_surface(SurfaceBuilder::new())
            .add_surface_from_scene(&scene)
            .build();
        info!("Ok, {} surfels", surface.samples.len());

        self.scene_directory = PathBuf::from(scene_obj_file_path);
        self.scene_directory.pop();
        self.scene = scene;
        self.surface = Some(surface);

        self.dump_surfels();

        self
    }

    fn dump_surfels(&self) {
        if let Some(surfel_obj_path) = self.surfel_obj_path.as_ref() {
            info!("Writing surface model to {:?}...", surfel_obj_path);
            let mut obj_file = fs::File::create(surfel_obj_path).unwrap();
            match self.surface.as_ref().unwrap().dump(&mut obj_file) {
                Ok(_) => info!("Ok"),
                Err(_) => info!("Failed")
            }
        }
    }

    pub fn hit_map_path<S : Into<PathBuf>>(mut self, path: S) -> SimulationBuilder {
        self.hit_map_path = Some(path.into());
        self
    }

    pub fn surfel_obj_path<S : Into<PathBuf>>(mut self, path: S) -> SimulationBuilder {
        self.surfel_obj_path = Some(path.into());
        self
    }

    pub fn ton_to_surface_interaction_weight(mut self, ton_to_surface_interaction_weight: f32) -> SimulationBuilder {
        self.ton_to_surface_interaction_weight = ton_to_surface_interaction_weight;
        self
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

    pub fn add_effect_blend(mut self, substance_idx: usize, subject_material_name: &str, subject_material_map: &str, blend_towards_tex_file: &str, output_directory: &str) -> SimulationBuilder {
        self.effects.push(
            Box::new(
                Blend::new(substance_idx, &self.scene_directory, subject_material_name, subject_material_map, blend_towards_tex_file, &Path::new(output_directory))
            )
        );

        self
    }

    pub fn add_effect_density_map(mut self, map_width: usize, map_height: usize, output_directory: &str) -> SimulationBuilder {
        self.effects.push(
            Box::new(
                DensityMap::new(map_width, map_height, output_directory)
            )
        );

        self
    }

    pub fn add_scene_sink_obj_mtl(mut self, obj_file_path: &str, mtl_file_path: &str) -> SimulationBuilder {
        self.scene_sinks.push(Box::new(
            MtlSink::new(mtl_file_path)
        ));
        self.scene_sinks.push(Box::new(
            ObjSink::new(obj_file_path, Some(mtl_file_path))
        ));

        self
    }

    pub fn build(self) -> Simulation {
        Simulation::new(
            self.scene,
            self.surface.unwrap(),
            self.ton_to_surface_interaction_weight,
            self.iterations,
            self.sources,
            self.effects,
            self.scene_sinks,
            self.hit_map_path
        )
    }
}
