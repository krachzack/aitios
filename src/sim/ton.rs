
use ::cgmath::Vector3;
use ::cgmath::InnerSpace;
use ::rand;

use std::f32::consts::PI;

pub struct Ton {
    /// Probability of moving further in a straight line
    #[allow(dead_code)]
    pub p_straight: f32,
    /// Probability of moving further in a piecewise approximated
    /// parabolic trajectory
    #[allow(dead_code)]
    pub p_parabolic: f32,
    /// Probability of moving tangently
    #[allow(dead_code)]
    pub p_flow: f32,
    /// Determines the radius around a ton where it interacts with surface elements.
    pub interaction_radius: f32,
    /// Amount of substances currently being carried by this ton
    pub substances: Vec<f32>
}

#[derive(Clone)]
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
    }
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
    /// Amount of substances initially carried by tons emitted by this source
    substances: Vec<f32>,
    emission_count: u32
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
    interaction_radius: f32
}

impl TonSource {
    /// Generates a new gammaton with associated ray origin and ray direction
    pub fn emit<'a>(&'a self) -> Box<Iterator<Item = (Ton, Vector3<f32>, Vector3<f32>)> + 'a> {
        let p_straight = self.p_straight;
        let p_parabolic = self.p_parabolic;
        let p_flow = self.p_flow;
        let interaction_radius = self.interaction_radius;
        let substances = self.substances.clone();
        let shape = self.shape.clone();

        let emissions = (0..self.emission_count).map(
            move |_| match shape {
                Shape::Point { position } => (
                    Ton {
                        p_straight,
                        p_parabolic,
                        p_flow,
                        interaction_radius,
                        substances: substances.clone()
                    },
                    position.clone(),
                    // Random position on the unit sphere
                    Vector3::new(
                        rand::random::<f32>() - 0.5,
                        rand::random::<f32>() - 0.5,
                        rand::random::<f32>() - 0.5
                    ).normalize()
                ),
                Shape::Hemisphere { center, radius } => {
                    let unit = sample_unit_hemisphere();
                    let origin = center + radius * unit;
                    // REVIEW wait, should they really all be flying towards the center?
                    let direction = -unit;

                    (
                        Ton {
                            p_straight,
                            p_parabolic,
                            p_flow,
                            interaction_radius,
                            substances: substances.clone()
                        },
                        origin,
                        direction
                    )
                }
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
            interaction_radius: 0.1
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

    pub fn emission_count(mut self, emission_count: u32) -> TonSourceBuilder {
        self.emission_count = emission_count;
        self
    }

    pub fn interaction_radius(mut self, interaction_radius: f32) -> TonSourceBuilder {
        self.interaction_radius = interaction_radius;
        self
    }

    pub fn build(self) -> TonSource {
        TonSource {
            shape: self.shape,
            p_straight: self.p_straight,
            p_parabolic: self.p_parabolic,
            p_flow: self.p_flow,
            interaction_radius: self.interaction_radius,
            substances: self.substances,
            emission_count: self.emission_count
        }
    }
}
