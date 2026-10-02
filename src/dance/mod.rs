mod dancer;
mod metadata;
mod shape;

use std::collections::HashMap;

pub use dancer::*;
pub use metadata::*;
pub use shape::*;

use crate::{
    coordinates::Coordinates,
    iterator::{DancerMovement, PathSegment},
};

#[derive(Debug, Clone)]
pub struct DanceScript {
    pub meta: Metadata,
    pub dancers: HashMap<DancerId, Dancer>,
    pub shapes: HashMap<ShapeId, ShapeDefinition>,
    pub start: Start,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct Start {
    pub instructions: Vec<StartInstruction>,
}

impl Start {
    pub fn get_position(
        &self,
        dancer: DancerId,
        shapes: &HashMap<ShapeId, ShapeDefinition>,
    ) -> Option<Coordinates> {
        self.instructions
            .iter()
            .find_map(|ins| ins.get_position(dancer, shapes))
    }
}

#[derive(Debug, Clone)]
pub enum StartInstruction {
    Shape {
        dancers: Vec<DancerId>,
        shape: ShapeId,
    },
    Position {
        dancer: DancerId,
        position: PositionExpression,
    },
}

impl StartInstruction {
    pub fn get_position(
        &self,
        dancer_id: DancerId,
        shapes: &HashMap<ShapeId, ShapeDefinition>,
    ) -> Option<Coordinates> {
        match self {
            StartInstruction::Shape { dancers, shape } => {
                let shape = shapes.get(shape).expect("shape exists");
                dancers
                    .iter()
                    .position(|d| *d == dancer_id)
                    .map(|i| shape.position(i as u64, dancers.len() as u64))
            }
            StartInstruction::Position { dancer, position } => {
                (dancer_id == *dancer).then_some(position.get(shapes))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Beat(pub u64, pub f64);

#[derive(Debug, Clone)]
pub struct Step {
    pub beat: Beat,
    pub instructions: Vec<Instruction>,
}

impl Step {
    pub fn pertains_to(&self, dancer_id: DancerId) -> bool {
        self.instructions
            .iter()
            .any(|ins| ins.pertains_to(dancer_id))
    }

    pub fn get(
        &self,
        dancer_id: DancerId,

        origin: Coordinates,
        next_instruction: Option<Beat>,

        metadata: &Metadata,
        shapes: &HashMap<ShapeId, ShapeDefinition>,
    ) -> Option<DancerMovement> {
        self.instructions.iter().find_map(|ins| {
            ins.get(
                dancer_id,
                self.beat,
                origin,
                next_instruction,
                metadata,
                shapes,
            )
        })
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub movement: Movement,
    pub until: Option<Beat>,
}

impl Instruction {
    fn pertains_to(&self, dancer_id: DancerId) -> bool {
        match &self.movement {
            Movement::ToPosition { dancer, .. } => *dancer == dancer_id,
            Movement::ByDelta { dancers, .. } | Movement::Rotate { dancers, .. } => {
                dancers.contains(dancer_id)
            }
            Movement::AlongShape { dancers, .. } => dancers.contains(&dancer_id),
        }
    }

    fn get(
        &self,
        dancer_id: DancerId,

        start: Beat,
        origin: Coordinates,
        next_instruction: Option<Beat>,

        metadata: &Metadata,
        shapes: &HashMap<ShapeId, ShapeDefinition>,
    ) -> Option<DancerMovement> {
        let end = self
            .until
            .or(next_instruction)
            .map(|e| metadata.beat_to_time(&e))
            .expect("either a duration or a following instruction");
        let start = metadata.beat_to_time(&start);
        let duration = end - start;

        let segment = match &self.movement {
            Movement::ToPosition { dancer, position } => {
                if *dancer != dancer_id {
                    return None;
                }

                let coordinates = position.get(shapes);
                PathSegment::Linear {
                    from: origin,
                    to: coordinates,
                }
            }

            Movement::ByDelta {
                dancers,
                delta,
                repeat,
            } => {
                if !dancers.contains(dancer_id) {
                    return None;
                }

                let full_delta = match repeat {
                    Some(r) => *delta * (duration / r),
                    None => *delta,
                };

                PathSegment::Linear {
                    from: origin,
                    to: origin + full_delta,
                }
            }

            Movement::AlongShape { dancers, shape } => {
                let index = dancers.iter().position(|d| *d == dancer_id)?;
                let shape = shapes.get(shape).expect("shape exists");
                let position = shape.position(index as u64, dancers.len() as u64);

                PathSegment::Linear {
                    from: origin,
                    to: position,
                }
            }

            Movement::Rotate {
                dancers,
                origin,
                angle,
            } => todo!(),
        };

        Some(DancerMovement {
            start,
            end,
            segment,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Movement {
    ToPosition {
        dancer: DancerId,
        position: PositionExpression,
    },
    ByDelta {
        dancers: DancerSpecifier,
        delta: Coordinates,
        repeat: Option<f64>,
    },
    AlongShape {
        dancers: Vec<DancerId>,
        shape: ShapeId,
    },
    Rotate {
        dancers: DancerSpecifier,
        origin: Coordinates,
        angle: f64,
    },
}

#[derive(Debug, Clone)]
pub enum PositionExpression {
    Shape(ShapeId, u64, u64),
    Literal(Coordinates),
}

impl PositionExpression {
    fn get(&self, shapes: &HashMap<ShapeId, ShapeDefinition>) -> Coordinates {
        match self {
            PositionExpression::Shape(shape_id, index, positions) => {
                let shape = shapes.get(shape_id).expect("shape exists");
                shape.position(*index - 1, *positions)
            }
            PositionExpression::Literal(coordinates) => *coordinates,
        }
    }
}

pub enum IndividualMovement {
    Absolute(Coordinates),
    Relative(Coordinates),
}
