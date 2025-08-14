use rocket::post;
use rocket::serde::json::Json;

use crate::models::fleet::{Fleet, MoveFleetData, MoveFleetResponse, save_fleet};
use crate::models::position::Position;
use crate::models::response::ApiResponse;
use crate::models::settings::GameSettings;
use crate::models::star_system::StarSystem;
use crate::models::settings::load_settings;

fn validate_galaxy_bounds(_target_pos: &Position, _settings: &GameSettings) -> Result<(), String> {
	Ok(())
}

fn clamp_to_galaxy(pos: &Position, settings: &GameSettings) -> Position {
	let max = settings.map_width as i32;
	let min = -max;
	Position { x: pos.x.clamp(min, max), y: pos.y.clamp(min, max), z: pos.z.clamp(min, max) }
}

#[allow(dead_code)]
fn compute_system_half_size(_system: &StarSystem) -> i32 { 0 }

fn distance(a: &Position, b: &Position) -> f64 {
	let dx = (b.x - a.x) as f64;
	let dy = (b.y - a.y) as f64;
	let dz = (b.z - a.z) as f64;
	(dx * dx + dy * dy + dz * dz).sqrt()
}

fn make_breakdown(scale_factor: f64, in_exit: f64, deep: f64, in_entry: f64) -> crate::models::fleet::DistanceBreakdown {
	let total_raw = in_exit + deep + in_entry;
	let total_scaled = (in_exit + in_entry) * scale_factor + deep;
	crate::models::fleet::DistanceBreakdown {
		in_system_exit: in_exit,
		deep_space: deep,
		in_system_entry: in_entry,
		total_raw,
		scale_factor,
		total_scaled,
	}
}

fn point_in_cube(p: &Position, center: &Position, half: i32) -> bool {
	let min_x = center.x - half;
	let max_x = center.x + half;
	let min_y = center.y - half;
	let max_y = center.y + half;
	let min_z = center.z - half;
	let max_z = center.z + half;
	p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y && p.z >= min_z && p.z <= max_z
}

fn line_cube_intersection(p0: &Position, p1: &Position, center: &Position, half: i32) -> Option<(f64, f64, Position, Position)> {
	let min = ((center.x - half) as f64, (center.y - half) as f64, (center.z - half) as f64);
	let max = ((center.x + half) as f64, (center.y + half) as f64, (center.z + half) as f64);
	let p0f = (p0.x as f64, p0.y as f64, p0.z as f64);
	let p1f = (p1.x as f64, p1.y as f64, p1.z as f64);
	let d = (p1f.0 - p0f.0, p1f.1 - p0f.1, p1f.2 - p0f.2);

	let mut tmin = 0.0f64;
	let mut tmax = 1.0f64;
	for i in 0..3 {
		let (p0c, dc, minc, maxc) = match i { 0 => (p0f.0, d.0, min.0, max.0), 1 => (p0f.1, d.1, min.1, max.1), _ => (p0f.2, d.2, min.2, max.2) };
		if dc.abs() < 1e-9 { if p0c < minc || p0c > maxc { return None; } } else {
			let mut t1 = (minc - p0c) / dc; let mut t2 = (maxc - p0c) / dc; if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
			if t1 > tmin { tmin = t1; } if t2 < tmax { tmax = t2; } if tmin > tmax { return None; }
		}
	}
	if tmin > 1.0 || tmax < 0.0 { return None; }
	let clamp = |t: f64| -> f64 { if t < 0.0 { 0.0 } else if t > 1.0 { 1.0 } else { t } };
	let t_entry = clamp(tmin);
	let t_exit = clamp(tmax);
	let entry = Position { x: (p0f.0 + d.0 * t_entry).round() as i32, y: (p0f.1 + d.1 * t_entry).round() as i32, z: (p0f.2 + d.2 * t_entry).round() as i32 };
	let exit = Position { x: (p0f.0 + d.0 * t_exit).round() as i32, y: (p0f.1 + d.1 * t_exit).round() as i32, z: (p0f.2 + d.2 * t_exit).round() as i32 };
	Some((t_entry, t_exit, entry, exit))
}

fn handle_system_movement(mut fleet: Fleet, target_pos: Position, system: &StarSystem, system_id: usize) -> Result<(MoveFleetResponse, Fleet), String> {
	println!("Handling system movement:");
	println!("  Fleet position: ({}, {}, {})", fleet.position.x, fleet.position.y, fleet.position.z);
	println!("  Target position: ({}, {}, {})", target_pos.x, target_pos.y, target_pos.z);
	println!("  System position: ({}, {}, {})", system.position.x, system.position.y, system.position.z);

	let settings_for_sys = load_settings().map_err(|e| e.to_string())?;
	let system_max = settings_for_sys.map_width as i32;
	let system_min = -system_max;

	let local_tx = target_pos.x - system.position.x;
	let local_ty = target_pos.y - system.position.y;
	let local_tz = target_pos.z - system.position.z;

	if local_tx < system_min || local_tx > system_max || local_ty < system_min || local_ty > system_max || local_tz < system_min || local_tz > system_max {
		println!("Target is on or outside system bounds (local coords: {}, {}, {}). Triggering system exit.", local_tx, local_ty, local_tz);
		return handle_system_exit(fleet, &target_pos, system, system_max);
	}

	println!("Performing local move within system {}", system_id);
	let curr_local = if let Some(lp) = &fleet.local_position { Position { x: lp.x, y: lp.y, z: lp.z } } else { Position { x: fleet.position.x - system.position.x, y: fleet.position.y - system.position.y, z: fleet.position.z - system.position.z } };
	let target_local = Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z };
	let local_distance = distance(&curr_local, &target_local);

	let local_p = Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z };
	fleet.position = system.position.clone();
	fleet.local_position = Some(local_p.clone());
	fleet.last_move_distance = Some(local_distance);
	fleet.current_system_id = Some(system_id);

	for ship in &mut fleet.ships { ship.position = system.position.clone(); }

	let response = MoveFleetResponse {
		status: "success".to_string(),
		message: if (local_distance - 0.0).abs() < 1e-9 { "Fleet already at target; no movement".to_string() } else { "Fleet moved successfully within system".to_string() },
		encounters: Vec::new(),
		current_position: system.position.clone(),
		target_position: system.position.clone(),
		remaining_distance: 0.0,
		current_system_id: Some(system_id),
		breakdown: Some(make_breakdown(1.0 / (load_settings().map_err(|e| e.to_string())?.map_width as f64), local_distance, 0.0, 0.0)),
		local_current_position: Some(Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z }),
		local_target_position: Some(Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z }),
	};

	Ok((response, fleet))
}

fn handle_system_exit(mut fleet: Fleet, target_pos: &Position, system: &StarSystem, system_max: i32) -> Result<(MoveFleetResponse, Fleet), String> {
	println!("Handling system exit:");
	println!("  Fleet position: ({}, {}, {})", fleet.position.x, fleet.position.y, fleet.position.z);
	println!("  Target position: ({}, {}, {})", target_pos.x, target_pos.y, target_pos.z);
	println!("  System position: ({}, {}, {})", system.position.x, system.position.y, system.position.z);

	let start_pos = fleet.position.clone();

	let local_tx = target_pos.x - system.position.x;
	let local_ty = target_pos.y - system.position.y;
	let local_tz = target_pos.z - system.position.z;

	let dir_x = if local_tx != 0 { local_tx.signum() } else { 0 };
	let dir_y = if local_ty != 0 { local_ty.signum() } else { 0 };
	let dir_z = if local_tz != 0 { local_tz.signum() } else { 0 };

	let (dir_x, dir_y, dir_z) = if dir_x == 0 && dir_y == 0 && dir_z == 0 {
		let cur_lx = fleet.position.x - system.position.x;
		let cur_ly = fleet.position.y - system.position.y;
		let cur_lz = fleet.position.z - system.position.z;
		let dx_edge = if cur_lx >= 0 { system_max - cur_lx } else { system_max + cur_lx.abs() };
		let dy_edge = if cur_ly >= 0 { system_max - cur_ly } else { system_max + cur_ly.abs() };
		let dz_edge = if cur_lz >= 0 { system_max - cur_lz } else { system_max + cur_lz.abs() };
		if dx_edge <= dy_edge && dx_edge <= dz_edge { (if cur_lx >= 0 { 1 } else { -1 }, 0, 0) } else if dy_edge <= dz_edge { (0, if cur_ly >= 0 { 1 } else { -1 }, 0) } else { (0, 0, if cur_lz >= 0 { 1 } else { -1 }) }
	} else { (dir_x, dir_y, dir_z) };

	let exit_x = system.position.x + dir_x * (system_max + 1);
	let exit_y = system.position.y + dir_y * (system_max + 1);
	let exit_z = system.position.z + dir_z * (system_max + 1);

	println!("Calculated exit position: ({}, {}, {})", exit_x, exit_y, exit_z);

	let exit_position = Position { x: exit_x, y: exit_y, z: exit_z };
	let in_exit = distance(&start_pos, &exit_position);
	let settings = load_settings().map_err(|e| e.to_string())?;
	let exit_position = clamp_to_galaxy(&exit_position, &settings);
	fleet.position = exit_position.clone();
	fleet.current_system_id = None;
	fleet.last_move_distance = Some(1.0);
	for ship in &mut fleet.ships { ship.position = exit_position.clone(); }

	let response = MoveFleetResponse {
		status: "transition_exit".to_string(),
		message: format!("Fleet exited the star system at coordinates ({}, {}, {})", exit_x, exit_y, exit_z),
		encounters: Vec::new(),
		current_position: exit_position.clone(),
		target_position: exit_position,
		remaining_distance: 0.0,
		current_system_id: None,
		breakdown: Some(make_breakdown(1.0 / (settings.map_width as f64), in_exit, 0.0, 0.0)),
		local_current_position: None,
		local_target_position: None,
	};

	Ok((response, fleet))
}

fn handle_deep_space_movement(mut fleet: Fleet, start_pos: Position, target_pos: Position, game_world: &[StarSystem]) -> Result<(MoveFleetResponse, Fleet), String> {
	println!("Handling deep space movement:");
	println!("  Start position: ({}, {}, {})", start_pos.x, start_pos.y, start_pos.z);
	println!("  Target position: ({}, {}, {})", target_pos.x, target_pos.y, target_pos.z);
	println!("  Number of systems in game world: {}", game_world.len());

	let deep_distance_full = distance(&start_pos, &target_pos);

	let settings_for_entry = load_settings().map_err(|e| e.to_string())?;
	let system_half = settings_for_entry.map_width as i32;

	for (index, system) in game_world.iter().enumerate() {
		println!("Checking system {} at position ({}, {}, {})", index, system.position.x, system.position.y, system.position.z);
		if target_pos.x == system.position.x && target_pos.y == system.position.y && target_pos.z == system.position.z {
			println!("Fleet entering System {} at galaxy coordinates ({}, {}, {})", index, target_pos.x, target_pos.y, target_pos.z);
			let entry_point = if let Some((_te, _tx, entryp, _exitp)) = line_cube_intersection(&start_pos, &target_pos, &system.position, system_half) { entryp } else { if point_in_cube(&start_pos, &system.position, system_half) { start_pos.clone() } else { system.position.clone() } };
			println!("Deterministic entry point: ({}, {}, {})", entry_point.x, entry_point.y, entry_point.z);
			let deep_distance = distance(&start_pos, &entry_point);
			fleet.position = entry_point.clone();
			fleet.current_system_id = Some(system.id);
			fleet.last_move_distance = Some(deep_distance);
			for ship in &mut fleet.ships { ship.position = entry_point.clone(); }
			let in_entry = 0.0;
			let scale = 1.0 / (load_settings().map_err(|e| e.to_string())?.map_width as f64);
			let response = MoveFleetResponse { status: "transition_entry".to_string(), message: format!("Fleet entered System {} at coordinates ({}, {}, {})", index, entry_point.x, entry_point.y, entry_point.z), encounters: vec![], current_position: entry_point, target_position: target_pos, remaining_distance: 0.0, current_system_id: Some(system.id), breakdown: Some(make_breakdown(scale, 0.0, deep_distance, in_entry)), local_current_position: Some(Position { x: 0, y: 0, z: 0 }), local_target_position: Some(Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z }) };
			return Ok((response, fleet));
		}
	}

	println!("No system entry detected, completing deep space movement");
	fleet.position = target_pos.clone();
	fleet.current_system_id = None;
	fleet.last_move_distance = Some(deep_distance_full);
	for ship in &mut fleet.ships { ship.position = target_pos.clone(); }
	let response = MoveFleetResponse { status: "success".to_string(), message: "Fleet completed deep space movement successfully".to_string(), encounters: vec![], current_position: target_pos.clone(), target_position: target_pos, remaining_distance: 0.0, current_system_id: None, breakdown: Some(make_breakdown(1.0 / (load_settings().map_err(|e| e.to_string())?.map_width as f64), 0.0, deep_distance_full, 0.0)), local_current_position: None, local_target_position: None };
	Ok((response, fleet))
}

#[post("/fleet/<owner_id>/<fleet_number>/move", format = "json", data = "<data>")]
pub fn move_fleet(owner_id: String, fleet_number: usize, data: Json<MoveFleetData>) -> Json<ApiResponse<MoveFleetResponse>> {
	println!("--- Starting Fleet Movement Operation ---");
	println!("  Fleet: Fleet_{}_{}", owner_id, fleet_number);
	println!("  Target Position: ({}, {}, {})", data.x, data.y, data.z);
	println!("  Intent: space={:?}, system_id={:?}, planet_id={:?}", data.space, data.system_id, data.planet_id);

	let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
	let result: Result<MoveFleetResponse, String> = (|| {
		let settings = load_settings().map_err(|e| e.to_string())?;
		let mut game_world = Vec::new();
		let star_dir = crate::models::game_state::game_data_path(&settings.game_id, &["star_systems"]);
		if star_dir.exists() {
			println!("  Loading star systems from {}", star_dir.display());
			for entry in std::fs::read_dir(&star_dir).map_err(|e| e.to_string())? {
				let path = entry.map_err(|e| e.to_string())?.path();
				if path.extension().and_then(|s| s.to_str()) == Some("json") {
					if let Ok(system) = crate::models::game_state::load_json::<StarSystem>(&path) { game_world.push(system); }
				}
			}
			if let Ok(mut guard) = crate::GLOBAL_GAME_WORLD.lock() { *guard = game_world.clone(); }
		} else if settings.game_id == "test_game" {
			let system_path = crate::models::game_state::game_data_path(&settings.game_id, &["star_systems", "Star_System_0.json"]);
			if let Ok(system) = crate::models::game_state::load_json::<StarSystem>(&system_path) { game_world = vec![system]; }
		} else {
			game_world = crate::models::game_world::get_global_game_world();
		}
		println!("  Game world contains {} systems", game_world.len());

		let initial_fleet = match crate::models::fleet::load_fleet(&fleet_name) {
			Ok(Some(fleet)) => fleet,
			Ok(None) => return Err(format!("Fleet '{}' not found", fleet_name)),
			Err(e) => return Err(format!("Failed to load fleet: {}", e)),
		};
		println!("  Loaded fleet at position ({}, {}, {})", initial_fleet.position.x, initial_fleet.position.y, initial_fleet.position.z);

		let target_pos = Position { x: data.x, y: data.y, z: data.z };
		let start_pos = initial_fleet.position.clone();
		validate_galaxy_bounds(&target_pos, &settings)?;

		fn resolve_system<'a>(world: &'a [StarSystem], sid: usize) -> Option<(usize, &'a StarSystem)> {
			if let Some(pos) = world.iter().position(|s| s.id == sid) { Some((pos, &world[pos])) } else { world.get(sid).map(|s| (sid, s)) }
		}

		let (response, updated_fleet) = {
			if let Some(planet_id) = data.planet_id {
				let system_id = data.system_id.ok_or_else(|| "planet_id provided without system_id".to_string())?;
				let (resolved_index, system) = resolve_system(&game_world, system_id).ok_or_else(|| format!("System {} not found", system_id))?;
				println!("  Planet intent → target system: id={} idx={} name={}", system_id, resolved_index, system.star.name);
				let planet = system.planets.get(planet_id).ok_or_else(|| format!("Planet {} not found in system {}", planet_id, system_id))?;
				let planet_galaxy_pos = Position { x: system.position.x + planet.position.x, y: system.position.y + planet.position.y, z: system.position.z + planet.position.z };
				let scale = 1.0 / (settings.map_width as f64);
				let target_half = settings.map_width as i32;
				let in_target_via_id = initial_fleet.current_system_id == Some(resolved_index);
				if in_target_via_id {
					let local_curr = if let Some(lp) = &initial_fleet.local_position { Position { x: lp.x, y: lp.y, z: lp.z } } else { Position { x: initial_fleet.position.x - system.position.x, y: initial_fleet.position.y - system.position.y, z: initial_fleet.position.z - system.position.z } };
					let local_tgt = planet.position.clone();
					let in_exit = distance(&local_curr, &local_tgt);
					let (_r, mut f) = handle_system_movement(initial_fleet, planet_galaxy_pos.clone(), system, resolved_index)?;
					f.last_move_distance = Some(in_exit);
					let mut resp = _r; resp.breakdown = Some(make_breakdown(scale, in_exit, 0.0, 0.0)); (resp, f)
				} else {
					let start = initial_fleet.position.clone();
					let target_center = system.position.clone();
					let curr_center_opt = if let Some(cid) = initial_fleet.current_system_id { game_world.get(cid).map(|cs| cs.position.clone()) } else { None };
					let (exit_point, in_exit) = if let Some(curr_center) = curr_center_opt.clone() {
						if let Some(_cs) = game_world.iter().find(|s| s.position == curr_center) {
							let curr_half = target_half;
							if point_in_cube(&start, &curr_center, curr_half) {
								if let Some((_te, _tx, _entry, exitp)) = line_cube_intersection(&start, &target_center, &curr_center, curr_half) { let d = distance(&start, &exitp); (exitp, d) } else { (start.clone(), 0.0) }
							} else { (start.clone(), 0.0) }
						} else { (start.clone(), 0.0) }
					} else { (start.clone(), 0.0) };
					let deep = distance(&exit_point, &target_center);
					let entry_point = if let Some((_te2, _tx2, entryp, _)) = line_cube_intersection(&exit_point, &target_center, &system.position, target_half) { entryp } else { target_center.clone() };
					let in_entry = distance(&entry_point, &planet_galaxy_pos);
					let fmid = { let mut temp = initial_fleet; temp.position = entry_point.clone(); temp.current_system_id = Some(system.id); temp.last_move_distance = Some(deep); temp };
					let (ri, ffinal) = handle_system_movement(fmid, planet_galaxy_pos.clone(), system, resolved_index)?;
					let mut resp = ri; resp.breakdown = Some(make_breakdown(scale, in_exit, deep, in_entry)); if deep > 0.0 || in_exit > 0.0 || in_entry > 0.0 { resp.message = "Fleet traveled between systems and arrived at target within system".to_string(); }
					resp.local_current_position = Some(Position { x: planet_galaxy_pos.x - system.position.x, y: planet_galaxy_pos.y - system.position.y, z: planet_galaxy_pos.z - system.position.z });
					resp.local_target_position = resp.local_current_position.clone();
					(resp, ffinal)
				}
			} else if matches!(data.space, Some(crate::models::fleet::MovementSpace::System)) {
				let system_id = data.system_id.ok_or_else(|| "space=system requires system_id".to_string())?;
				let (resolved_index, system) = resolve_system(&game_world, system_id).ok_or_else(|| format!("System {} not found", system_id))?;
				println!("  System intent → target system: id={} idx={} name={}", system_id, resolved_index, system.star.name);
				let scale = 1.0 / (settings.map_width as f64);
				let target_half = settings.map_width as i32;
				let in_target_via_id = initial_fleet.current_system_id == Some(resolved_index);
				if in_target_via_id {
					let local_curr = if let Some(lp) = &initial_fleet.local_position { Position { x: lp.x, y: lp.y, z: lp.z } } else { Position { x: initial_fleet.position.x - system.position.x, y: initial_fleet.position.y - system.position.y, z: initial_fleet.position.z - system.position.z } };
					let local_tgt = Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z };
					let in_exit = distance(&local_curr, &local_tgt);
					let (r, mut f) = handle_system_movement(initial_fleet, target_pos, system, resolved_index)?;
					f.last_move_distance = Some(in_exit);
					let mut resp = r; resp.breakdown = Some(make_breakdown(scale, in_exit, 0.0, 0.0)); resp.local_current_position = Some(Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z }); resp.local_target_position = resp.local_current_position.clone(); (resp, f)
				} else {
					let start = initial_fleet.position.clone();
					let target_center = system.position.clone();
					let curr_center_opt = if let Some(cid) = initial_fleet.current_system_id { game_world.get(cid).map(|cs| cs.position.clone()) } else { game_world.iter().find(|s| point_in_cube(&start, &s.position, target_half)).map(|s| s.position.clone()) };
					let (exit_point, in_exit) = if let Some(curr_center) = curr_center_opt.clone() {
						if let Some(_cs) = game_world.iter().find(|s| s.position == curr_center) {
							let curr_half = target_half;
							if point_in_cube(&start, &curr_center, curr_half) {
								if let Some((_te, _tx, _entry, exitp)) = line_cube_intersection(&start, &target_center, &curr_center, curr_half) { let d = distance(&start, &exitp); (exitp, d) } else { (start.clone(), 0.0) }
							} else { (start.clone(), 0.0) }
						} else { (start.clone(), 0.0) }
					} else { (start.clone(), 0.0) };
					let deep = distance(&exit_point, &target_center);
					let entry_point = if let Some((_te2, _tx2, entryp, _)) = line_cube_intersection(&exit_point, &target_center, &system.position, target_half) { entryp } else { target_center.clone() };
					let in_entry = distance(&entry_point, &target_pos);
					let fmid = { let mut temp = initial_fleet; temp.position = entry_point.clone(); temp.current_system_id = Some(system.id); temp.last_move_distance = Some(deep); temp };
					let (ri, ffinal) = handle_system_movement(fmid, target_pos, system, resolved_index)?;
					let mut resp = ri; resp.breakdown = Some(make_breakdown(scale, in_exit, deep, in_entry)); if deep > 0.0 || in_exit > 0.0 || in_entry > 0.0 { resp.message = "Fleet traveled between systems and arrived at target within system".to_string(); }
					resp.local_current_position = Some(Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z }); resp.local_target_position = resp.local_current_position.clone(); (resp, ffinal)
				}
			} else if matches!(data.space, Some(crate::models::fleet::MovementSpace::Galaxy)) {
				println!("  Galaxy intent → deep-space move to ({}, {}, {})", target_pos.x, target_pos.y, target_pos.z);
				handle_deep_space_movement(initial_fleet, start_pos, target_pos, &game_world)?
			} else if data.system_id.is_some() {
				let system_id = data.system_id.unwrap();
				let (resolved_index, system) = resolve_system(&game_world, system_id).ok_or_else(|| format!("System {} not found", system_id))?;
				println!("  system_id-only intent → treat as in-system target: id={} idx={} name={} to ({}, {}, {})", system_id, resolved_index, system.star.name, target_pos.x, target_pos.y, target_pos.z);
				let scale = 1.0 / (settings.map_width as f64);
				let target_half = settings.map_width as i32;
				let in_target_via_id = initial_fleet.current_system_id == Some(resolved_index);
				if in_target_via_id {
					let local_curr = if let Some(lp) = &initial_fleet.local_position { Position { x: lp.x, y: lp.y, z: lp.z } } else { Position { x: initial_fleet.position.x - system.position.x, y: initial_fleet.position.y - system.position.y, z: initial_fleet.position.z - system.position.z } };
					let local_tgt = Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z };
					let in_exit = distance(&local_curr, &local_tgt);
					let (r, mut f) = handle_system_movement(initial_fleet, target_pos, system, resolved_index)?;
					f.last_move_distance = Some(in_exit);
					let mut resp = r; resp.breakdown = Some(make_breakdown(scale, in_exit, 0.0, 0.0)); resp.local_current_position = Some(Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z }); resp.local_target_position = resp.local_current_position.clone(); (resp, f)
				} else {
					let start = initial_fleet.position.clone(); let target_center = system.position.clone(); let curr_center_opt = if let Some(cid) = initial_fleet.current_system_id { game_world.get(cid).map(|cs| cs.position.clone()) } else { game_world.iter().find(|s| point_in_cube(&start, &s.position, target_half)).map(|s| s.position.clone()) };
					let (exit_point, in_exit) = if let Some(curr_center) = curr_center_opt.clone() { if let Some(_cs) = game_world.iter().find(|s| s.position == curr_center) { let curr_half = target_half; if point_in_cube(&start, &curr_center, curr_half) { if let Some((_te, _tx, _entry, exitp)) = line_cube_intersection(&start, &target_center, &curr_center, curr_half) { let d = distance(&start, &exitp); (exitp, d) } else { (start.clone(), 0.0) } } else { (start.clone(), 0.0) } } else { (start.clone(), 0.0) } } else { (start.clone(), 0.0) };
					let deep = distance(&exit_point, &target_center);
					let entry_point = if let Some((_te2, _tx2, entryp, _)) = line_cube_intersection(&exit_point, &target_center, &system.position, target_half) { entryp } else { target_center.clone() };
					let in_entry = distance(&entry_point, &target_pos);
					let fmid = { let mut temp = initial_fleet; temp.position = entry_point.clone(); temp.current_system_id = Some(system.id); temp.last_move_distance = Some(deep); temp };
					let (ri, ffinal) = handle_system_movement(fmid, target_pos, system, resolved_index)?; let mut resp = ri; resp.breakdown = Some(make_breakdown(scale, in_exit, deep, in_entry)); if deep > 0.0 || in_exit > 0.0 || in_entry > 0.0 { resp.message = "Fleet traveled between systems and arrived at target within system".to_string(); }
					resp.local_current_position = Some(Position { x: target_pos.x - system.position.x, y: target_pos.y - system.position.y, z: target_pos.z - system.position.z }); resp.local_target_position = resp.local_current_position.clone(); (resp, ffinal)
				}
			} else {
				if let Some(system_id) = initial_fleet.current_system_id { if let Some(system) = game_world.get(system_id) { println!("  Default branch → in-system move in idx={} name={}", system_id, system.star.name); handle_system_movement(initial_fleet, target_pos, system, system_id)? } else { println!("  Default branch → deep-space move (current system not found) to ({}, {}, {})", target_pos.x, target_pos.y, target_pos.z); handle_deep_space_movement(initial_fleet, start_pos, target_pos, &game_world)? } } else { println!("  Default branch → deep-space move to ({}, {}, {})", target_pos.x, target_pos.y, target_pos.z); handle_deep_space_movement(initial_fleet, start_pos, target_pos, &game_world)? }
			}
		};

		println!("Saving final fleet state for {}", updated_fleet.name);
		save_fleet(&updated_fleet)?;
		println!("Fleet saved successfully.");
		Ok(response)
	})();

	match result {
		Ok(response) => { let message = response.message.clone(); println!("  Movement successful: {}", message); ApiResponse::success(response, message) },
		Err(e) => { println!("  Movement failed: {}", e); ApiResponse::error(e) }
	}
}


