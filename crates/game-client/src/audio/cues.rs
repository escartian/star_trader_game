//! Which UI sound, if any, a frame should play. Every egui widget click plays
//! the click cue without per-widget code; map canvases opt out (`quiet_clicks`)
//! so a click on empty space stays silent, and mark their own `select` cue when
//! a click lands on a star, planet or fleet. At most one cue plays per frame.
use super::Effect;
use egui::{Context, Id};

const CLICK_KEY: &str = "ui-click-sound";
const SELECT_KEY: &str = "ui-select-sound";
const QUIET_KEY: &str = "ui-quiet-widgets";

/// A control was pressed (also detected automatically for egui widgets).
pub fn mark_click(ctx: &Context) {
    ctx.data_mut(|data| data.insert_temp(Id::new(CLICK_KEY), true));
}

/// A star, planet, body or fleet was picked on a map.
pub fn mark_select(ctx: &Context) {
    ctx.data_mut(|data| data.insert_temp(Id::new(SELECT_KEY), true));
}

/// Clicks on widget `id` (a map canvas) play only what it marks itself.
pub fn quiet_clicks(ctx: &Context, id: Id) {
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Vec<Id>>(Id::new(QUIET_KEY))
            .push(id);
    });
}

/// The cue for this frame, clearing the marks. Call once, after the UI.
pub fn take_cue(ctx: &Context) -> Option<Effect> {
    let (select, click, quiet) = ctx.data_mut(|data| {
        (
            data.remove_temp::<bool>(Id::new(SELECT_KEY))
                .unwrap_or(false),
            data.remove_temp::<bool>(Id::new(CLICK_KEY))
                .unwrap_or(false),
            data.remove_temp::<Vec<Id>>(Id::new(QUIET_KEY))
                .unwrap_or_default(),
        )
    });
    let widget = ctx.interaction_snapshot(|snapshot| snapshot.clicked);
    let widget_click = widget.is_some_and(|id| !quiet.contains(&id));
    if select {
        Some(Effect::Select)
    } else if click || widget_click {
        Some(Effect::Click)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_beats_click_and_marks_clear_each_frame() {
        let ctx = Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            mark_click(ctx);
            mark_select(ctx);
            assert_eq!(take_cue(ctx), Some(Effect::Select));
            assert_eq!(take_cue(ctx), None);
            mark_click(ctx);
            assert_eq!(take_cue(ctx), Some(Effect::Click));
        });
    }
}
