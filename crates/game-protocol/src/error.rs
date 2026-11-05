use game_core::{economy::EconomyError, travel::TravelError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    Travel(TravelError),
    Economy(EconomyError),
}

impl From<TravelError> for CommandError {
    fn from(error: TravelError) -> Self {
        Self::Travel(error)
    }
}
impl From<EconomyError> for CommandError {
    fn from(error: EconomyError) -> Self {
        Self::Economy(error)
    }
}
impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Travel(error) => error.fmt(f),
            Self::Economy(error) => error.fmt(f),
        }
    }
}
