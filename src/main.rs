extern crate tobj;
extern crate cgmath;
extern crate rand;
extern crate image;

mod geom;
mod sim;

use sim::SimulationBuilder;

fn main() {
    let model_obj_path = "testdata/aperoom.obj";
    
    SimulationBuilder::new()
        .scene(model_obj_path)
        .add_point_source(&cgmath::Vector3::new(0.0, 4.0, 1.0))
        .iterations(1)
        .build()
        .run();
}
