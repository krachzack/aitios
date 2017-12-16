extern crate aitios;
#[macro_use] extern crate log;
extern crate simplelog;
extern crate chrono;

mod common;

use std::fs::create_dir;

#[test]
fn buddha_room_bronze_test() {
    let directory = common::prepare_test_directory("buddha_room_bronze_test");

    let mut obj_file = directory.clone();
    obj_file.push("buddha-scene-weathered");
    obj_file.set_extension("obj");
    let obj_file = obj_file.to_str().unwrap();

    let mut mtl_file = directory.clone();
    mtl_file.push("buddha-scene-weathered");
    mtl_file.set_extension("mtl");
    let mtl_file = mtl_file.to_str().unwrap();

    let mut density_map_output_directory = directory.clone();
    density_map_output_directory.push("density-maps");
    create_dir(&density_map_output_directory).unwrap();

    let density_map_output_directory = density_map_output_directory.to_str().unwrap();

    let blent_map_output_directory = directory.to_str().unwrap();

    let input_path = "testdata/buddha-scene";
    let model_obj_path = format!("{}/buddha-scene.obj", input_path);

    let mut hit_map_path = directory.clone();
    hit_map_path.push("interacted_surfels");
    hit_map_path.set_extension("obj");

    aitios::SimulationBuilder::new()
        .ton_to_surface_interaction_weight(1.0)
        .scene(
            &model_obj_path,
            |s| {
                s.surfels_per_sqr_unit(100.0)
                    .delta_straight(1.0)
                    .substances(&vec![0.0])
            }
        )
        //.scene_substances(&vec![0.0])
        .add_source(|s| {
            s.p_straight(1.0)
                .substances(&vec![1.0])
                .point_shaped(0.0, 1.0, 0.0)
                .emission_count(80000)
        })
        // TODO instead of changing a material, maybe we should change an object
        .add_effect_blend(
            0, // Index of substance that drives the blend
            "bronze", // material that gets changed
            "map_Kd", // map of the material that gets changed
            "testdata/buddha-scene/weathered_bronze.png",
            blent_map_output_directory
        )
        .add_effect_blend(
            0,
            "stone",
            "map_Kd",
            "testdata/buddha-scene/moss.png",
            blent_map_output_directory
        )
        //.add_effect_density_map(256, 256, density_map_output_directory)
        //.add_effect_density_map(512, 512, density_map_output_directory)
        .add_effect_density_map(1024, 1024, density_map_output_directory)
        .add_scene_sink_obj_mtl(obj_file, mtl_file)
        .hit_map_path(hit_map_path)
        .iterations(1)
        .build()
        .run();
}
