use std::collections::HashMap;

use derive_more::{Add, Div, Mul, Sub};

use crate::dance::DancerId;

#[derive(Debug, Clone, Copy, Add, Sub, Mul, Div)]
pub struct Coordinates {
    pub x: f64,
    pub y: f64,
}

impl Coordinates {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone)]
pub struct GroupCoordinates(pub HashMap<DancerId, Coordinates>);
