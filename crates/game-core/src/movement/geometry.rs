//! Checked integer geometry; rendering projections never enter these calculations.
use super::{MovementError, SpacePosition};
use crate::LocalPosition;

pub(super) fn local(position: LocalPosition) -> [i64; 3] {
    [position.x_km, position.y_km, position.z_km]
}
pub(super) fn position(values: [i64; 3]) -> LocalPosition {
    LocalPosition {
        x_km: values[0],
        y_km: values[1],
        z_km: values[2],
    }
}

pub(super) fn absolute(
    center: SpacePosition,
    offset: LocalPosition,
) -> Result<SpacePosition, MovementError> {
    let mut result = [0; 3];
    for (index, value) in local(offset).iter().enumerate() {
        result[index] = center.coordinates()[index]
            .checked_add(*value)
            .ok_or(MovementError::Coordinates)?;
    }
    Ok(SpacePosition {
        x_km: result[0],
        y_km: result[1],
        z_km: result[2],
    })
}

pub(super) fn distance_squared(a: [i64; 3], b: [i64; 3]) -> Result<u128, MovementError> {
    a.into_iter().zip(b).try_fold(0_u128, |sum, (a, b)| {
        let delta = (i128::from(a) - i128::from(b)).unsigned_abs();
        delta
            .checked_mul(delta)
            .and_then(|square| sum.checked_add(square))
            .ok_or(MovementError::Coordinates)
    })
}

pub(super) fn distance(a: [i64; 3], b: [i64; 3]) -> Result<u64, MovementError> {
    let squared = distance_squared(a, b)?;
    let floor = squared.isqrt();
    u64::try_from(floor + u128::from(floor * floor != squared))
        .map_err(|_| MovementError::Coordinates)
}

/// First cube face intersected by a ray from an interior point. Place one km beyond
/// that face; the other components use the same ray parameter (integer truncation).
pub(super) fn exit(
    start: LocalPosition,
    target: LocalPosition,
    half: i64,
) -> Result<LocalPosition, MovementError> {
    let (start, target) = (local(start), local(target));
    let mut crossing: Option<(usize, i128, i128)> = None;
    for axis in 0..3 {
        if target[axis].unsigned_abs() <= half as u64 {
            continue;
        }
        let face = if target[axis] > 0 { half } else { -half };
        let numerator = (i128::from(face) - i128::from(start[axis])).abs();
        let denominator = (i128::from(target[axis]) - i128::from(start[axis])).abs();
        if crossing.is_none_or(|(_, previous_num, previous_den)| {
            numerator * previous_den < previous_num * denominator
        }) {
            crossing = Some((axis, numerator, denominator));
        }
    }
    let (axis, numerator, denominator) = crossing.ok_or(MovementError::Coordinates)?;
    let mut result = [0; 3];
    for index in 0..3 {
        let value = i128::from(start[index])
            + (i128::from(target[index]) - i128::from(start[index])) * numerator / denominator;
        result[index] = i64::try_from(value).map_err(|_| MovementError::Coordinates)?;
    }
    result[axis] = if target[axis] > 0 {
        half + 1
    } else {
        -half - 1
    };
    Ok(position(result))
}

/// Near face when approaching the system center. A system coordinate is a target
/// anchor, not an instruction to fly through the star.
pub(super) fn entry(
    origin: SpacePosition,
    center: SpacePosition,
    half: i64,
) -> Result<LocalPosition, MovementError> {
    let delta: Vec<i128> = origin
        .coordinates()
        .into_iter()
        .zip(center.coordinates())
        .map(|(a, b)| i128::from(a) - i128::from(b))
        .collect();
    let largest = delta
        .iter()
        .map(|value| value.abs())
        .max()
        .ok_or(MovementError::Coordinates)?;
    if largest <= i128::from(half) {
        return Err(MovementError::WrongSpace);
    }
    let mut result = [0; 3];
    for axis in 0..3 {
        result[axis] = (delta[axis] * i128::from(half) / largest) as i64;
    }
    Ok(position(result))
}

/// Whether the segment touches the sphere of `radius` around `centre`.
pub(super) fn crosses_sphere(
    start: LocalPosition,
    end: LocalPosition,
    centre: LocalPosition,
    radius: i64,
) -> bool {
    let shift = |point: LocalPosition| {
        position(std::array::from_fn(|axis| {
            local(point)[axis] - local(centre)[axis]
        }))
    };
    crosses_star(shift(start), shift(end), radius)
}

/// Segment/sphere intersection using integer dot products, bounded local points.
pub(super) fn crosses_star(start: LocalPosition, end: LocalPosition, radius: i64) -> bool {
    let start = local(start).map(i128::from);
    let end = local(end).map(i128::from);
    let delta = std::array::from_fn::<_, 3, _>(|axis| end[axis] - start[axis]);
    let length: i128 = delta.iter().map(|value| value * value).sum();
    let projection: i128 = start.iter().zip(delta).map(|(a, b)| -a * b).sum();
    let square = |point: [i128; 3]| point.iter().map(|value| value * value).sum::<i128>();
    let radius = i128::from(radius).pow(2);
    if projection <= 0 || length == 0 {
        return square(start) <= radius;
    }
    if projection >= length {
        return square(end) <= radius;
    }
    square(start) * length - projection * projection <= radius * length
}
