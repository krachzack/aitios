extern crate aitios;
#[macro_use] extern crate log;
extern crate simplelog;
extern crate chrono;

mod common;

#[cfg_attr(not(feature = "expensive_tests"), ignore)]
#[test]
fn ramp_test() {
    let directory = common::prepare_test_directory("ramp");

    let obj_file = "ramp-weathered.obj";
    let mtl_file = "ramp-weathered.mtl";

    /*let mut density_map_output_directory = directory.clone();
    density_map_output_directory.push("substance-density-maps");
    create_dir(&density_map_output_directory).unwrap();

    let density_map_output_directory = density_map_output_directory.to_str().unwrap();

    let blent_map_output_directory = directory.to_str().unwrap();*/

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
                    .delta_parabolic(0.4) // up to two bounces
                    .delta_flow(0.1) // way more flow events
                    .substances(&vec![0.0, 0.0])
                    // Buddha and bunnies get water from gammatons, but no rust
                    .deposition_rates(vec![0.1, 0.0])
                    .override_material(
                        "stone",
                        |s| {
                            // Floor gets dissolved rust but no water
                            s.deposition_rates(vec![0.1, 0.05])
                        }
                    )
            }
        )
        .add_source(|s| {
            s.p_straight(0.0)
                .p_straight(0.0)
                .p_parabolic(0.2)
                .p_flow(0.8)
                .interaction_radius(0.06)
                .parabola_height(0.1)
                .flow_upward_offset(0.002)
                .flow_downward_pull(0.01)
                .substances(&vec![1.0, 0.0]) // gammatons carry water and no rust
                .pickup_rates(vec![0.0, 0.5]) // Gammatons pick up all the rust on contact
                .mesh_shaped("test-scenes/buddha-scene-ton-source-mesh/sky-disk.obj")
                .emission_count(100000)
        })
        // Water should slowly lead to rust accumulation
        // rust = rust + 0.15 * water
        .add_material_surfel_rule("bronze", 1, 0, 0.15)
        // And water also evaporates
        // water = water - 0.5 * water
        .add_global_surfel_rule(0, 0, -0.5)
        .substance_map_size(
            1,
            1024,
            1024
        )
        .add_effect_density_map()
        .add_effect_ramp()
        .add_effect_blend(
            vec![String::from("bronze"), String::from("stone"), String::from("iron")],
            "test-scenes/buddha-scene-iron-concrete/",
            "test-scenes/buddha-scene-iron-concrete/RustPlain018_COL_VAR1_1K.jpg"
        )
        .add_scene_sink_obj_mtl(obj_file, mtl_file)
        .output_path(directory)
        .hit_map_path(hit_map_path)
        .iterations(30)
        .build()
        .run();
}
