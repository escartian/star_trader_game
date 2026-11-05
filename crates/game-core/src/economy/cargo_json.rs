//! Store cargo by commodity name rather than expose array indices in player saves.
use super::Commodity;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

pub(crate) fn serialize<S: Serializer>(cargo: &[u32; 6], serializer: S) -> Result<S::Ok, S::Error> {
    Commodity::ALL
        .into_iter()
        .map(|kind| (kind, cargo[kind.index()]))
        .collect::<BTreeMap<_, _>>()
        .serialize(serializer)
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<[u32; 6], D::Error> {
    let values = BTreeMap::<Commodity, u32>::deserialize(deserializer)?;
    Ok(std::array::from_fn(|index| {
        values.get(&Commodity::ALL[index]).copied().unwrap_or(0)
    }))
}
