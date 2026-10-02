use crate::color::Color;

#[derive(Debug, Clone)]
pub struct Dancer {
    pub name: String,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub enum DancerSpecifier {
    Dancers(Vec<DancerId>),
    AllExcept(Vec<DancerId>),
}
impl DancerSpecifier {
    pub fn contains(&self, dancer_id: DancerId) -> bool {
        match self {
            DancerSpecifier::Dancers(ids) => ids.contains(&dancer_id),
            DancerSpecifier::AllExcept(ids) => !ids.contains(&dancer_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DancerId(pub u64);
