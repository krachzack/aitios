
use ::cgmath::Vector3;
use ::cgmath::InnerSpace;
use ::rand;

use ::geom::TriangleBins;
use ::geom::scene::{Scene, Vertex};

use std::f32::EPSILON;

pub struct Ton {
    /// Probability of moving further in a straight line
    pub p_straight: f32,
    /// Probability of moving further in a piecewise approximated
    /// parabolic trajectory
    pub p_parabolic: f32,
    /// Probability of moving tangently
    pub p_flow: f32,
    /// Determines the radius around a ton where it interacts with surface elements.
    pub interaction_radius: f32,
    /// Determines the height of a vertical bounce
    pub parabola_height: f32,
    /// Amount of substances currently being carried by this ton
    pub substances: Vec<f32>,
    /// Factor by which the gammaton picks up material from surfels
    pub pickup_rates: Vec<f32>
}

// TODO the sampling should be stratified, e.g. by subdividing the possible directions into patches and ensuring every one gets its turn
enum Shape {
    /// A point source shooting equally in all directions
    Point { position: Vector3<f32> },
    /// A hemispherical source aligned with the y axis shooting inward.
    Hemisphere {
        /// The center of the bottom disk of the hemisphere
        center: Vector3<f32>,
        /// Distance from the center for ray origins
        radius: f32
    },
    /// Shoots from the given mesh in interpolated normal direction
    Mesh { triangles: TriangleBins<Vertex> }
}

pub struct TonSource {
    /// Emission shape
    shape: Shape,
    /// Probability of moving further in a straight line for tons emitted by this source
    p_straight: f32,
    /// Probability of moving further in a piecewise approximated for tons emitted by this source
    /// parabolic trajectory
    p_parabolic: f32,
    /// Probability of moving tangently for tons emitted by this source
    p_flow: f32,
    /// Determines the radius around a ton where it interacts with surface elements.
    interaction_radius: f32,
    /// Determines the height of a vertical bounce
    parabola_height: f32,
    /// Amount of substances initially carried by tons emitted by this source
    substances: Vec<f32>,
    emission_count: u32,
    pickup_rates: Vec<f32>
}

pub struct TonSourceBuilder {
    /// Emission shape
    shape: Shape,
    /// Probability of moving further in a straight line for tons emitted by this source
    p_straight: f32,
    /// Probability of moving further in a piecewise approximated for tons emitted by this source
    /// parabolic trajectory
    p_parabolic: f32,
    /// Probability of moving tangently for tons emitted by this source
    p_flow: f32,
    /// Amount of substances initially carried by tons emitted by this source
    substances: Vec<f32>,
    emission_count: u32,
    pickup_rates: Vec<f32>,
    /// Determines the radius around a ton where it interacts with surface elements.
    interaction_radius: f32,
    /// Determines the height of a vertical bounce
    parabola_height: f32,
}

impl TonSource {
    /// Generates a new gammaton with associated ray origin and ray direction
    pub fn emit<'a>(&'a self) -> Box<Iterator<Item = (Ton, Vector3<f32>, Vector3<f32>)> + 'a> {
        let p_straight = self.p_straight;
        let p_parabolic = self.p_parabolic;
        let p_flow = self.p_flow;
        let interaction_radius = self.interaction_radius;
        let parabola_height = self.parabola_height;
        let substances = self.substances.clone();
        let pickup_rates = self.pickup_rates.clone();
        //let shape = self.shape.clone();

        let emissions = (0..self.emission_count).map(
            move |_| {
                let (origin, direction) = match &self.shape {
                    &Shape::Point { position } => (
                        position.clone(),
                        // Random position on the unit sphere
                        Vector3::new(
                            rand::random::<f32>() - 0.5,
                            rand::random::<f32>() - 0.5,
                            rand::random::<f32>() - 0.5
                        ).normalize()
                    ),
                    &Shape::Hemisphere { center, radius } => {
                        let unit = sample_unit_hemisphere();
                        let origin = center + radius * unit;
                        // REVIEW wait, should they really all be flying towards the center?
                        let direction = -unit;
                        (origin, direction)
                    },
                    &Shape::Mesh { ref triangles } => {
                        // Interpolate a vertex on a random position on a randomly selected triangle (weighted by area)
                        let vtx = triangles.sample().sample_vertex();
                        let direction = vtx.normal;
                        let origin = vtx.position + direction * EPSILON;
                        (origin, direction)
                    }
                };
                (
                    Ton {
                        p_straight,
                        p_parabolic,
                        p_flow,
                        interaction_radius,
                        parabola_height,
                        substances: substances.clone(),
                        pickup_rates: pickup_rates.clone()
                    },
                    origin,
                    direction
                )
            }
        );

        Box::new(emissions)
    }

    pub fn emission_count(&self) -> u32 {
        self.emission_count
    }
}

fn sample_unit_hemisphere() -> Vector3<f32> {
    // REVIEW this is certainly not uniform

    let x = rand::random::<f32>() - 0.5;
    let y = rand::random::<f32>() * 0.5;
    let z = rand::random::<f32>() - 0.5;

    Vector3::new(x, y, z).normalize()
}

impl TonSourceBuilder {
    pub fn new() -> TonSourceBuilder {
        TonSourceBuilder {
            p_straight: 0.0,
            p_parabolic: 0.0,
            p_flow: 0.0,
            substances: Vec::new(),
            shape: Shape::Point { position: Vector3::new(0.0, 0.0, 0.0) },
            emission_count: 10000,
            interaction_radius: 0.1,
            parabola_height: 0.05,
            pickup_rates: Vec::new()
        }
    }

    pub fn p_straight(mut self, p_straight: f32) -> TonSourceBuilder {
        self.p_straight = p_straight;
        self
    }

    #[allow(dead_code)]
    pub fn p_parabolic(mut self, p_parabolic: f32) -> TonSourceBuilder {
        self.p_parabolic = p_parabolic;
        self
    }

    #[allow(dead_code)]
    pub fn p_flow(mut self, p_flow: f32) -> TonSourceBuilder {
        self.p_flow = p_flow;
        self
    }

    pub fn substances(mut self, substances: &Vec<f32>) -> TonSourceBuilder {
        self.substances = substances.clone();
        self
    }

    pub fn point_shaped(mut self, pos_x: f32, pos_y: f32, pos_z: f32) -> TonSourceBuilder {
        self.shape = Shape::Point { position: Vector3::new(pos_x, pos_y, pos_z) };
        self
    }

    pub fn hemisphere_shaped(mut self, center: Vector3<f32>, radius: f32) -> TonSourceBuilder {
        self.shape = Shape::Hemisphere { center, radius };
        self
    }

    pub fn mesh_shaped(mut self, obj_file_path: &str) -> TonSourceBuilder {
        let scene = Scene::load_from_file(obj_file_path);

        self.shape = Shape::Mesh {
            triangles: TriangleBins::new(
                scene.triangles().collect(),
                32
            )
        };

        self
    }

    pub fn emission_count(mut self, emission_count: u32) -> TonSourceBuilder {
        self.emission_count = emission_count;
        self
    }

    pub fn interaction_radius(mut self, interaction_radius: f32) -> TonSourceBuilder {
        self.interaction_radius = interaction_radius;
        self
    }

    pub fn parabola_height(mut self, parabola_height: f32) -> TonSourceBuilder {
        self.parabola_height = parabola_height;
        self
    }

    pub fn pickup_rates<R : IntoIterator<Item = f32>> (mut self, pickup_rates: R) -> TonSourceBuilder {
        self.pickup_rates = pickup_rates.into_iter().collect();
        self
    }

    pub fn build(self) -> TonSource {
        assert_eq!(self.pickup_rates.len(), self.substances.len());

        TonSource {
            shape: self.shape,
            p_straight: self.p_straight,
            p_parabolic: self.p_parabolic,
            p_flow: self.p_flow,
            interaction_radius: self.interaction_radius,
            parabola_height: self.parabola_height,
            substances: self.substances,
            emission_count: self.emission_count,
            pickup_rates: self.pickup_rates
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_shoot_from_mesh() {
        let src = TonSourceBuilder::new()
            .p_flow(0.2)
            .emission_count(10)
            .mesh_shaped("test-scenes/buddha-scene-ton-source-mesh/buddha-scene-ton-source-sun.obj")
            .build();

        assert_eq!(src.emit().count(), 10);
        assert!(src.emit().all(|(ton, origin, direction)| ton.p_flow == 0.2 && origin.y > 0.1 && direction.y < 0.0));
    }
}
