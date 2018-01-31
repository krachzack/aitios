extern crate aitios;
#[macro_use] extern crate log;
extern crate simplelog;
extern crate chrono;

mod common;

#[cfg_attr(not(feature = "expensive_tests"), ignore)]
#[test]
/// Tests the behavior of gammatons on parabolic trajectories
fn bounce_test() {
    let directory = common::prepare_test_directory("bounce-test");

    let obj_file = "bounce-test-weathered.obj";
    let mtl_file = "bounce-test-weathered.mtl";

    let input_path = "test-scenes/buddha-scene-iron-concrete";
    let model_obj_path = format!("{}/buddha-scene-iron-concrete.obj", input_path);

    let mut hit_map_path = directory.clone();
    hit_map_path.push("interacted_surfels");
    hit_map_path.set_extension("obj");

    let mut surfels_path = directory.clone();
    surfels_path.push("surfels");
    surfels_path.set_extension("obj");

    aitios::SimulationBuilder::new()
        .surfel_obj_path(surfels_path)
        .scene(
            &model_obj_path,
            |s| {
                s.min_sample_distance(0.02)
                    .delta_straight(1.0)
                    .delta_parabolic(0.2) // up to five bounces
                    .delta_flow(0.3) // way more flow events
                    .substances(&vec![0.0])
                    .deposition_rates(vec![1.0])
            }
        )
        .add_environment_source(|s| {
            s.p_straight(0.0)
                .p_straight(0.0)
                .p_parabolic(1.0)
                .p_flow(0.0)
                .parabola_height(0.1)
                .interaction_radius(0.1)
                .substances(&vec![1.0]) // gammatons carry rust
                .pickup_rates(vec![1.0]) // Gammatons pick up all the rust on contact
                //.mesh_shaped("test-scenes/buddha-scene-ton-source-mesh/sky-disk.obj")
                .emission_count(10000)
        })
        .substance_map_size(
            0,
            1024,
            1024
        )
        .add_effect_density_map()
        .add_effect_blend(
            vec![String::from("bronze"), String::from("stone"), String::from("iron")],
            "test-scenes/buddha-scene-iron-concrete/",
            "test-scenes/buddha-scene-iron-concrete/RustPlain018_COL_VAR1_1K.jpg"
        )
        .add_scene_sink_obj_mtl(obj_file, mtl_file)
        .output_path(directory)
        .hit_map_path(hit_map_path)
        .iterations(5)
        .build()
        .run();
}
