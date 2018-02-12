extern crate aitios;
#[macro_use] extern crate log;
extern crate simplelog;
extern crate chrono;

mod common;

#[cfg_attr(not(feature = "expensive_tests"), ignore)]
#[test]
fn surfel_sampling_by_density_test() {
    let directory = common::prepare_test_directory("sampling-by-density");

    let obj_file = "multi-weathered-sampling-density.obj";
    let mtl_file = "multi-weathered-sampling-density.mtl";

    let input_path = "test-scenes/buddha-pedestal";
    let model_obj_path = format!("{}/buddha-pedestal.obj", input_path);

    let mut hit_map_path = directory.clone();
    hit_map_path.push("interacted_surfels_by_density");
    hit_map_path.set_extension("obj");

    let mut surfels_path = directory.clone();
    surfels_path.push("surfels_by_density");
    surfels_path.set_extension("obj");

    aitios::SimulationBuilder::new()
        .surfel_obj_path(surfels_path)
        .scene(
            &model_obj_path,
            |s| {
                s.sample_density(580.0)
                    .delta_straight(1.0)
                    .delta_parabolic(0.4) // up to two bounces
                    .delta_flow(0.05) // way more flow events
                    .substances(&vec![0.0, 0.0])
                    // Buddha and bunnies get water from gammatons, but no rust
                    .deposition_rates(vec![1.0, 0.0])
                    .override_material(
                        "Concrete",
                        |s| {
                            // Floor gets exaggerated dissolved rust but no water
                            s.deposition_rates(vec![0.0, 1.3])
                                .delta_straight(1.0)
                                .delta_parabolic(1.0)
                                .delta_flow(1.0)
                        }
                    )
            }
        )
        .add_source(|s| {
            s.p_straight(0.0)
                .p_straight(0.0)
                .p_parabolic(0.8)
                .p_flow(0.2)
                .interaction_radius(0.1)
                .parabola_height(0.08)
                .flow_upward_offset(0.002)
                .flow_downward_pull(0.01)
                .substances(&vec![1.0, 0.0]) // gammatons carry water and no rust
                .pickup_rates(vec![0.0, 1.0]) // Gammatons pick up all the rust on contact
                .mesh_shaped("test-scenes/buddha-scene-ton-source-mesh/sky-disk.obj")
                .emission_count(100000)
        })
        // Water should slowly lead to rust accumulation
        // rust = rust + 0.25 * water
        .add_material_surfel_rule("iron_buddha", 1, 0, 0.3)
        .add_material_surfel_rule("iron_bun_big", 1, 0, 0.3)
        .add_material_surfel_rule("iron_bun_small", 1, 0, 0.3)
        // And water also evaporates
        // water = water - 0.5 * water
        .add_global_surfel_rule(0, 0, -0.5)
        .substance_map_size(
            1,
            1024,
            1024
        )
        .add_effect_density_map()
        //.add_effect_ramp()
        .add_effect_blend(
            vec![String::from("Concrete"), String::from("iron_buddha"), String::from("iron_bun_big"), String::from("iron_bun_small")],
            "test-scenes/buddha-scene-iron-concrete/",
            "test-scenes/buddha-scene-iron-concrete/RustPlain018_COL_VAR1_1K.jpg"
        )
        .add_scene_sink_obj_mtl(obj_file, mtl_file)
        .output_path(directory.clone())
        .hit_map_path(hit_map_path)
        .iterations(3)
        .build()
        .run();
}
