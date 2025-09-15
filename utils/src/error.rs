#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrineTreeError {
    InvalidArgument,
    TreeFull,
    InvalidProof,
    ProofLength,
}

pub type ProgramResult = Result<(), BrineTreeError>;
