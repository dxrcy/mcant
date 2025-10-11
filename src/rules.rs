use mcrs::Block;

#[derive(Debug)]
pub struct Rule {
    pub from_block: Vec<Block>,
    pub from_state: Vec<State>,
    pub to_block: Block,
    pub to_facing: Rotation,
    pub to_state: State,
}

pub type State = String;

#[derive(Clone, Copy, Debug)]
pub enum Rotation {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Rotation {
    pub fn into_vec3(self) -> [i32; 3] {
        match self {
            Rotation::PosX => [1, 0, 0],
            Rotation::NegX => [-1, 0, 0],
            Rotation::PosY => [0, 1, 0],
            Rotation::NegY => [0, -1, 0],
            Rotation::PosZ => [0, 0, 1],
            Rotation::NegZ => [0, 0, -1],
        }
    }
}
