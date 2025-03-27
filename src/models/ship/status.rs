#[derive(Debug, Clone, PartialEq)]
pub enum ShipStatus {
    Active,
    Inactive,
    Damaged,
    Destroyed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CombatState {
    Ready,
    Engaged,
    Retreating,
    Disabled,
} 