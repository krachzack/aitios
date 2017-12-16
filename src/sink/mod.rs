
use ::geom::scene::Scene;

use std::fmt;
use std::error;
use std::result;
use std::io;

pub mod mtl;
pub mod obj;

pub type Result<T> = result::Result<T, Error>;

pub trait SceneSink {
    fn serialize(&self, scene: &Scene) -> Result<()>;
}

#[derive(Debug)]
pub enum Error {
    IO(io::Error)
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "Something bad happened"
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IO(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

