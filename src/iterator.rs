use std::collections::HashMap;

use crate::{
    coordinates::{Coordinates, GroupCoordinates},
    dance::{Beat, DanceScript, DancerId, Step},
};

struct PathwayBuilder<'a> {
    movements: Vec<DancerMovement>,
    last_step: Option<&'a Step>,
    dance: &'a DanceScript,
    dancer_id: DancerId,
    start_pos: Coordinates,
}

impl<'a> PathwayBuilder<'a> {
    pub fn new(dance: &'a DanceScript, dancer_id: DancerId) -> Self {
        let start_pos = dance
            .start
            .get_position(dancer_id, &dance.shapes)
            .expect("dancer has a start position");

        Self {
            dance,
            last_step: None,
            movements: vec![],
            dancer_id,
            start_pos,
        }
    }

    pub fn push(&mut self, step: &'a Step) {
        if let Some(last_movement) = self.get_movement(Some(step.beat)) {
            self.movements.push(last_movement);
        }

        self.last_step = Some(step);
    }

    fn get_movement(&self, next_beat: Option<Beat>) -> Option<DancerMovement> {
        let last = self.last_step?;

        let origin = self
            .movements
            .last()
            .map_or(self.start_pos, |m| m.segment.destination());

        let step = last
            .get(
                self.dancer_id,
                origin,
                next_beat,
                &self.dance.meta,
                &self.dance.shapes,
            )
            .expect("instructions for this dancer");

        Some(step)
    }

    pub fn build(mut self) -> Pathway {
        if let Some(last_movement) = self.get_movement(None) {
            self.movements.push(last_movement);
        }

        Pathway {
            steps: self.movements,
            step_index: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Pathway {
    steps: Vec<DancerMovement>,
    step_index: usize,
}

impl Pathway {
    // fn current_step(&self) -> &DancerMovement {
    //     &self.steps[self.step_index]
    // }
    //
    // fn increment_step(&mut self) -> &DancerMovement {
    //     self.step_index += 1;
    //     self.current_step()
    // }

    fn advance_to(&mut self, time: f64) -> Coordinates {
        while self.step_index < self.steps.len() - 1 && self.steps[self.step_index].end < time {
            self.step_index += 1;
        }

        // calculate the current position
        let movement = &self.steps[self.step_index];
        let progress = ((time - movement.start) / (movement.end - movement.start)).clamp(0.0, 1.0);
        movement.segment.lerp(progress)
    }
}

type Angle = f64;

#[derive(Debug, Clone)]
pub enum PathSegment {
    Linear {
        from: Coordinates,
        to: Coordinates,
    },
    Arc {
        pivot: Coordinates,
        radius: f64,
        start: Angle,
        angle: Angle,
    },
}

impl PathSegment {
    fn destination(&self) -> Coordinates {
        self.lerp(1.0)
    }

    fn lerp(&self, progress: f64) -> Coordinates {
        match self {
            PathSegment::Linear { from, to } => *from * (1.0 - progress) + *to * progress,
            PathSegment::Arc {
                pivot,
                radius,
                start,
                angle,
            } => {
                let deg = start + angle * progress;
                *pivot + Coordinates::new(deg.sin(), deg.cos()) * *radius
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DancerMovement {
    pub start: f64,
    pub end: f64,
    pub segment: PathSegment,
}

#[derive(Debug, Clone)]
pub struct DanceIterator {
    dance: DanceScript,
    pathways: HashMap<DancerId, Pathway>,
}

impl From<DanceScript> for DanceIterator {
    fn from(dance: DanceScript) -> Self {
        let mut pathways = HashMap::new();

        for dancer_id in dance.dancers.keys() {
            let mut pathway = PathwayBuilder::new(&dance, *dancer_id);

            for step in dance.steps.iter() {
                if step.pertains_to(*dancer_id) {
                    pathway.push(step);
                }
            }

            pathways.insert(*dancer_id, pathway.build());
        }

        Self { dance, pathways }
    }
}

impl DanceIterator {
    pub fn dance(&self) -> &DanceScript {
        &self.dance
    }

    pub fn advance_to(&mut self, time: f64) -> GroupCoordinates {
        let map = self
            .pathways
            .iter_mut()
            .map(|(id, pathway)| (*id, pathway.advance_to(time)))
            .collect();

        GroupCoordinates(map)
    }
}
