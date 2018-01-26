extern crate aitios;
#[macro_use] extern crate log;
extern crate simplelog;
extern crate chrono;

mod common;

#[cfg_attr(not(feature = "expensive_tests"), ignore)]
#[test]
/// Tests the behavior of gammatons on parabolic trajectories
fn flow_test() {
    let directory = common::prepare_test_directory("flow-test");

    let obj_file = "flow-test-weathered.obj";
    let mtl_file = "flow-test-weathered.mtl";

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
                s.min_sample_distance(0.01)
                    .delta_straight(1.0)
                    .delta_parabolic(0.1) // up to two bounces
                    .delta_flow(0.05) // way more flow events
                    .substances(&vec![0.0])
                    .deposition_rates(vec![0.04])
                    .override_material(
                        "stone",
                        |s| {
                            // Floor gets little rust
                            s.deposition_rates(vec![0.005])
                        }
                    )
            }
        )
        .add_source(|s| {
            s.p_straight(0.0)
                .p_straight(0.0)
                .p_parabolic(0.0)
                .p_flow(1.0)
                .interaction_radius(0.03)
                .parabola_height(0.05)
                .substances(&vec![1.0])
                .pickup_rates(vec![0.1])
                .mesh_shaped("test-scenes/buddha-scene-ton-source-mesh/sky-disk.obj")
                .emission_count(200000)
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
