use std::f64::consts::PI;

use crate::coordinates::Coordinates;

#[derive(Debug, Clone)]
pub enum ShapeDefinition {
    Line {
        from: Coordinates,
        through: Coordinates,
    },
    Horseshoe {
        about: Coordinates,
        closure: f64,
        width: f64,
        depth: f64,
    },
}

impl ShapeDefinition {
    pub fn position(&self, index: u64, slots: u64) -> Coordinates {
        let amount = index as f64 / (slots - 1) as f64;
        match self {
            ShapeDefinition::Line { from, through } => *from * (1.0 - amount) + *through * amount,
            ShapeDefinition::Horseshoe {
                about,
                closure,
                width,
                depth,
            } => {
                let amount = amount * (1.0 - closure) + 0.5;
                let t = amount * 2.0 * PI;
                *about + Coordinates::new(0.5 * width * t.cos(), 0.5 * depth * t.sin())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeId(pub u64);
