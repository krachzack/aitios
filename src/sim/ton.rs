
use ::cgmath::Vector3;
use ::cgmath::InnerSpace;
use ::rand;

pub struct Ton {
    /// Probability of moving further in a straight line
    p_straight: f32,
    /// Probability of moving further in a piecewise approximated
    /// parabolic trajectory
    p_parabolic: f32,
    /// Probability of moving tangently
    p_flow: f32,
    /// Amount of materials currently being carried by this ton
    materials: Vec<f32>
}

#[derive(Clone)]
enum Shape {
    Point { position: Vector3<f32> }
}

pub struct TonSource {
    /// Probability of moving further in a straight line
    p_straight: f32,
    /// Probability of moving further in a piecewise approximated
    /// parabolic trajectory
    p_parabolic: f32,
    /// Probability of moving tangently
    p_flow: f32,
    /// Amount of materials currently being carried by this ton
    materials: Vec<f32>,
    /// Emission shape
    shape: Shape
}

impl TonSource {
    pub fn new() -> TonSource {
        TonSource {
            p_straight: 0.0,
            p_parabolic: 0.0,
            p_flow: 0.0,
            materials: Vec::new(),
            shape: Shape::Point { position: Vector3::new(0.0, 0.0, 0.0) }
        }
    }

    pub fn p_straight(&mut self, p_straight: f32) -> &mut TonSource {
        self.p_straight = p_straight;
        self
    }

    pub fn p_parabolic(&mut self, p_parabolic: f32) -> &mut TonSource {
        self.p_parabolic = p_parabolic;
        self
    }

    pub fn p_flow(&mut self, p_flow: f32) -> &mut TonSource {
        self.p_flow = p_flow;
        self
    }

    pub fn materials(&mut self, materials: &Vec<f32>) -> &mut TonSource {
        self.materials = materials.clone();
        self
    }

    pub fn point_shaped(&mut self, position: &Vector3<f32>) -> &mut TonSource {
        self.shape = Shape::Point { position: position.clone() };
        self
    }

    /// Generates a new gammaton with associated ray origin and ray direction
    pub fn emit<'a>(&'a self, count: u32) -> Box<Iterator<Item = (Ton, Vector3<f32>, Vector3<f32>)> + 'a> {
        let p_straight = self.p_straight;
        let p_parabolic = self.p_parabolic;
        let p_flow = self.p_flow;
        let materials = self.materials.clone();
        let shape = self.shape.clone();

        let emissions = (0..count).map(
            move |_| match shape {
                Shape::Point { position } => (
                    Ton {
                        p_straight,
                        p_parabolic,
                        p_flow,
                        materials: materials.clone()
                    },
                    position.clone(),
                    // Random position on the unit sphere
                    Vector3::new(
                        rand::random::<f32>() - 0.5,
                        rand::random::<f32>() - 0.5,
                        rand::random::<f32>() - 0.5
                    ).normalize()
                )
            }
        );

        Box::new(emissions)
    }
}
