use crate::models::resource::ResourceType;

#[derive(Debug, PartialEq)]
pub enum TradeAction {
    Buy { resource_type: ResourceType, quantity: u32 },
    Sell { resource_type: ResourceType, quantity: u32 },
}

#[derive(Debug, PartialEq)]
pub enum TradeResult {
    Success,
    InsufficientResourcesBuying,
    InsufficientResourcesSelling,
    InvalidResource,
    TransactionFailed,
    TraderNotFound,
}