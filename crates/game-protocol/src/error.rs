use game_core::{economy::EconomyError, fleet::FleetError, travel::TravelError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    Travel(TravelError),
    Economy(EconomyError),
    Fleet(FleetError),
    Movement(game_core::movement::MovementError),
    Navigation(game_core::navigation::NavigationError),
    Job(game_core::jobs::JobError),
}

impl From<FleetError> for CommandError {
    fn from(error: FleetError) -> Self {
        Self::Fleet(error)
    }
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
            Self::Fleet(error) => error.fmt(f),
            Self::Movement(error) => error.fmt(f),
            Self::Navigation(error) => error.fmt(f),
            Self::Job(error) => f.write_str(error.message()),
        }
    }
}

impl From<game_core::fleet::FleetMovementError> for CommandError {
    fn from(error: game_core::fleet::FleetMovementError) -> Self {
        match error {
            game_core::fleet::FleetMovementError::Navigation(error) => Self::Navigation(error),
            game_core::fleet::FleetMovementError::Movement(error) => Self::Movement(error),
            game_core::fleet::FleetMovementError::Travel(error) => Self::Travel(error),
            game_core::fleet::FleetMovementError::Fleet(error) => Self::Fleet(error),
        }
    }
}

impl From<game_core::navigation::NavigationError> for CommandError {
    fn from(error: game_core::navigation::NavigationError) -> Self {
        Self::Navigation(error)
    }
}

impl From<game_core::jobs::JobError> for CommandError {
    fn from(error: game_core::jobs::JobError) -> Self {
        Self::Job(error)
    }
}
