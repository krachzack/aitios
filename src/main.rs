extern crate tobj;
extern crate cgmath;
extern crate rand;
extern crate image;

mod geom;
mod sim;

use sim::SimulationBuilder;
use cgmath::Vector3;

fn main() {
    let model_obj_path = "testdata/aperoom.obj";
    
    SimulationBuilder::new()
        .scene(model_obj_path)
        //.scene_substances(&vec![0.0])
        .add_source(|s| {
            s.p_straight(1.0)
                .substances(&vec![1.0])
                .point_shaped(&Vector3::new(0.0, 4.0, 1.0))
                .emission_count(30000)
        })
        .iterations(1)
        .build()
        .run();
}
