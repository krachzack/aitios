
#[macro_use]
extern crate log;
extern crate tobj;
extern crate cgmath;
extern crate rand;
extern crate image;
extern crate kdtree;
extern crate float_extras;

mod geom;
mod sim;
mod sink;

pub use sim::{Simulation, SimulationBuilder};
