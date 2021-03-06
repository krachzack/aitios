
use std::fs;
use std::io::prelude::*;
use std::io;
use std::path::PathBuf;
use std::iter;

use ::geom::surf::{Surface, SurfaceBuilder};
use ::geom::scene::Scene;
use ::geom::spatial::Spatial;

use ::sink::*;
use ::sink::obj::ObjSink;
use ::sink::mtl::MtlSink;

use ::cgmath::Vector4;
use ::cgmath::Vector3;
use ::cgmath::prelude::*;

use ::image;

use super::sim::Simulation;
use super::ton::{TonSourceBuilder, TonSource};
use super::effect::{Effect, SubstanceMapper, Sampling, SubstanceColorEffect, SubstanceMapMaterialEffect, SurfelRule, Blend, Ramp, RampSegment};

/// Builds a simulation according to provided parameters and closures.
///
/// # Examples
///
/// The following example builds and runs a simulation on
/// `"test-scenes/buddha-scene/buddha-scene.obj"` and adds a sink for
/// the weathered scene.
///
/// ```rust,no_run
/// use aitios::SimulationBuilder;
///
/// SimulationBuilder::new()
///     .scene(
///         "test-scenes/buddha-scene/buddha-scene.obj",
///         |s| {
///             s.delta_straight(1.0)
///                 .substances(&vec![0.0])
///         }
///     )
///     .add_source(|s| {
///         s.p_straight(1.0)
///             .substances(&vec![1.0])
///             .point_shaped(0.0, 2.0, 0.0)
///             .emission_count(40000)
///     })
///     .add_scene_sink_obj_mtl(
///         "output/buddha-scene-weathered.obj",
///         "output/buddha-scene-weathered.mtl"
///     )
///     .iterations(1)
///     .build()
///     .run();
/// ```
pub struct SimulationBuilder {
    // TODO this should hold SceneBuilder and SurfaceBuilder
    scene: Scene,
    scene_directory: PathBuf,
    surface: Option<Surface>,
    iterations: u32,
    sources: Vec<TonSource>,
    effects: Vec<Box<Effect>>,
    substance_map_effects: Vec<Box<SubstanceMapMaterialEffect>>,
    scene_sinks: Vec<Box<SceneSink>>,
    hit_map_path: Option<PathBuf>,
    surfel_obj_path: Option<PathBuf>,
    substance_idx: usize,
    substance_map_width: usize,
    substance_map_height: usize,
    substance_map_sampling: Sampling,
    output_path: Option<PathBuf>
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
            substance_map_effects: Vec::new(),
            scene_sinks: Vec::new(),
            hit_map_path: None,
            surfel_obj_path: None,
            substance_idx: 0,
            substance_map_width: 4096,
            substance_map_height: 4096,
            substance_map_sampling: Sampling::NearestTriangle,
            output_path: None
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

    pub fn output_path<S : Into<PathBuf>>(mut self, path: S) -> SimulationBuilder {
        self.output_path = Some(path.into());
        self
    }

    pub fn hit_map_path<S : Into<PathBuf>>(mut self, path: S) -> SimulationBuilder {
        self.hit_map_path = Some(path.into());
        self
    }

    pub fn surfel_obj_path<S : Into<PathBuf>>(mut self, path: S) -> SimulationBuilder {
        self.surfel_obj_path = Some(path.into());
        self
    }

    pub fn iterations(mut self, iterations: u32) -> SimulationBuilder {
        self.iterations = iterations;
        self
    }

    pub fn add_environment_source<F>(mut self, build: F) -> SimulationBuilder
        where F: FnOnce(TonSourceBuilder) -> TonSourceBuilder
    {
        let aabb = self.scene.bounds();
        let bottom_center = 0.5 * (aabb.min + Vector3::new(aabb.max.x, aabb.min.y, aabb.max.z));
        let radius = aabb.max.distance(aabb.min);

        self.sources.push(
            build(TonSourceBuilder::new())
                .hemisphere_shaped(bottom_center, radius)
                .build()
        );

        self
    }

    pub fn add_source<F>(mut self, build: F) -> SimulationBuilder
        where F: FnOnce(TonSourceBuilder) -> TonSourceBuilder
    {

        self.sources.push(
            build(TonSourceBuilder::new()).build()
        );

        self
    }

    /*pub fn add_effect_blend(mut self, substance_idx: usize, subject_material_name: &str, subject_material_map: &str, blend_towards_tex_file: &str, output_directory: &str) -> SimulationBuilder {
        self.substance_map_effects.push(
            Box::new(
                Blend::new(substance_idx, &self.scene_directory, subject_material_name, subject_material_map, blend_towards_tex_file, &Path::new(output_directory))
            )
        );

        self
    }*/

    pub fn substance_map_size(mut self, substance_idx: usize, width: usize, height: usize) -> SimulationBuilder {
        self.substance_idx = substance_idx;
        self.substance_map_width = width;
        self.substance_map_height = height;

        self
    }

    pub fn substance_map_rasterize(mut self, padding: f32) -> SimulationBuilder {
        self.substance_map_sampling = Sampling::Rasterization(padding);
        self
    }

    /*pub fn substance_map_gather_radius(mut self, radius: f32) -> SimulationBuilder {
        self.substance_map_sampling = Sampling::Rasterization(0.0);
        self
    }*/

    /// Adds a new surfel rule that gets performed before creating substance maps for surfels of all materials
    pub fn add_global_surfel_rule(mut self, write_substance_idx: usize, read_substance_idx: usize, rate: f32) -> SimulationBuilder {
        self.effects.push(
            Box::new(SurfelRule::new(write_substance_idx, read_substance_idx, rate, iter::empty::<String>()))
        );

        self
    }

    pub fn add_material_surfel_rule(mut self, applicable_material: &str, write_substance_idx: usize, read_substance_idx: usize, rate: f32) -> SimulationBuilder {
        self.effects.push(
            Box::new(SurfelRule::new(write_substance_idx, read_substance_idx, rate, iter::once(applicable_material)))
        );

        self
    }

    pub fn add_effect_density_map(mut self) -> SimulationBuilder {
        self.substance_map_effects.push(
            Box::new(
                SubstanceColorEffect::new(
                    Vector4::new(1.0, 1.0, 1.0, 1.0), // substance = 0
                    Vector4::new(0.0, 0.0, 0.0, 1.0), // substance = 1
                    Vector4::new(0.0, 0.0, 1.0, 1.0),  // substance = NaN
                )
            )
        );

        self
    }

    pub fn add_effect_ramp(mut self) -> SimulationBuilder {
        let material_names = vec![String::from("bronze"), String::from("stone"), String::from("iron")];
        let segments = vec![
            RampSegment::new(0.0, 0.1, None, None),
            RampSegment::new(0.1, 0.25, None, Some(image::open("test-scenes/buddha-scene-iron-concrete/RustPlain018_COL_VAR1_1K.jpg").unwrap())),
            RampSegment::new(0.25, 1.0, Some(image::open("test-scenes/buddha-scene-iron-concrete/RustPlain018_COL_VAR1_1K.jpg").unwrap()), Some(image::open("test-scenes/buddha-scene-iron-concrete/RustPlain018_COL_VAR1_1K.jpg").unwrap()))
        ];

        let ramp = Box::new(Ramp::new(material_names, PathBuf::from("test-scenes/buddha-scene-iron-concrete/"), segments));

        self.substance_map_effects.push(ramp);

        self
    }

    pub fn add_effect_blend<P>(mut self, target_material_names: Vec<String>, texture_base_path: P, blend_target_image: P) -> SimulationBuilder
        where P : Into<PathBuf>
    {
        self.substance_map_effects.push(
            Box::new(
                Blend::new(target_material_names, texture_base_path, &blend_target_image.into())
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

    pub fn build(mut self) -> Simulation {
        let substance_mapper = SubstanceMapper::new(
            self.substance_idx, self.substance_map_sampling, self.substance_map_width, self.substance_map_height, self.substance_map_effects
        );

        self.effects.push(
            Box::new(substance_mapper)
        );

        Simulation::new(
            self.scene,
            self.surface.unwrap(),
            self.iterations,
            self.sources,
            self.effects,
            self.scene_sinks,
            self.output_path.unwrap(),
            self.hit_map_path
        )
    }
}
