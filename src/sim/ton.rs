
use ::cgmath::Vector3;
use ::cgmath::InnerSpace;
use ::rand;

pub struct Ton {
    /// Probability of moving further in a straight line
    p_straight: f32,
    /// Probability of moving further in a piecewise approximated
    /// parabolic trajectory
    #[allow(dead_code)]
    p_parabolic: f32,
    /// Probability of moving tangently
    #[allow(dead_code)]
    p_flow: f32,
    /// Amount of materials currently being carried by this ton
    pub materials: Vec<f32>
}

#[derive(Clone)]
enum Shape {
    Point { position: Vector3<f32> }
}

pub struct TonSource {
    /// Emission shape
    shape: Shape,
    /// Probability of moving further in a straight line for tons emitted by this source
    p_straight: f32,
    /// Probability of moving further in a piecewise approximated for tons emitted by this source
    /// parabolic trajectory
    #[allow(dead_code)]
    p_parabolic: f32,
    /// Probability of moving tangently for tons emitted by this source
    #[allow(dead_code)]
    p_flow: f32,
    /// Amount of materials initially carried by tons emitted by this source
    materials: Vec<f32>,
    emission_count: u32
}

pub struct TonSourceBuilder {
    /// Emission shape
    shape: Shape,
    /// Probability of moving further in a straight line for tons emitted by this source
    p_straight: f32,
    /// Probability of moving further in a piecewise approximated for tons emitted by this source
    /// parabolic trajectory
    #[allow(dead_code)]
    p_parabolic: f32,
    /// Probability of moving tangently for tons emitted by this source
    #[allow(dead_code)]
    p_flow: f32,
    /// Amount of materials initially carried by tons emitted by this source
    materials: Vec<f32>,
    emission_count: u32
}

impl TonSource {
    /// Generates a new gammaton with associated ray origin and ray direction
    pub fn emit<'a>(&'a self) -> Box<Iterator<Item = (Ton, Vector3<f32>, Vector3<f32>)> + 'a> {
        let p_straight = self.p_straight;
        let p_parabolic = self.p_parabolic;
        let p_flow = self.p_flow;
        let materials = self.materials.clone();
        let shape = self.shape.clone();

        let emissions = (0..self.emission_count).map(
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

    pub fn emission_count(&self) -> u32 {
        self.emission_count
    }
}

impl TonSourceBuilder {
    pub fn new() -> TonSourceBuilder {
        TonSourceBuilder {
            p_straight: 0.0,
            p_parabolic: 0.0,
            p_flow: 0.0,
            materials: Vec::new(),
            shape: Shape::Point { position: Vector3::new(0.0, 0.0, 0.0) },
            emission_count: 10000
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

    pub fn materials(mut self, materials: &Vec<f32>) -> TonSourceBuilder {
        self.materials = materials.clone();
        self
    }

    pub fn point_shaped(mut self, position: &Vector3<f32>) -> TonSourceBuilder {
        self.shape = Shape::Point { position: position.clone() };
        self
    }

    pub fn emission_count(mut self, emission_count: u32) -> TonSourceBuilder {
        self.emission_count = emission_count;
        self
    }

    pub fn build(self) -> TonSource {
        TonSource {
            shape: self.shape,
            p_straight: self.p_straight,
            p_parabolic: self.p_parabolic,
            p_flow: self.p_flow,
            materials: self.materials,
            emission_count: self.emission_count
        }
    }
}
