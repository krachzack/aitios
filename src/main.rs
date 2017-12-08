extern crate aitios;

fn main() {
    let model_obj_path = "testdata/ape_room.obj";
    
    aitios::SimulationBuilder::new()
        .scene(
            model_obj_path,
            |s| {
                s.surfels_per_sqr_unit(5000.0)
                    .delta_straight(1.0)
                    .substances(&vec![0.0])
            }
        )
        //.scene_substances(&vec![0.0])
        .add_source(|s| {
            s.p_straight(1.0)
                .substances(&vec![1.0])
                .point_shaped(0.0, 4.0, 1.0)
                .emission_count(30000)
        })
        // TODO instead of changing a material, maybe we should change an object
        .add_effect_blend(
            0, // Index of substance that drives the blend
            "green_plastic", // material that gets changed
            "map_Kd", // map of the material that gets changed
            "green_plastic_maximum_weathered.png"
        )
        .iterations(1)
        .build()
        .run();
}
