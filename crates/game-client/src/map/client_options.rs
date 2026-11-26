//! Carries out the player's audio and display options each frame: which music
//! plays and how loud, interface scale, animations and fullscreen. Display
//! options are sent only when they change, so a capture run's settings (and
//! egui's own zoom keys) hold until the player edits an option.
use super::Map;
use crate::audio::{Music, Track, music_gain};

impl Map {
    /// Desktop launches hand over the opened audio device; tests stay silent.
    pub fn set_music(&mut self, music: Music) {
        self.music = music;
    }

    pub(super) fn apply_client_options(&mut self, ctx: &egui::Context) {
        let options = self.session.client_options();
        let previous = self.applied_options.unwrap_or_default();
        if options.ui_scale != previous.ui_scale {
            ctx.set_zoom_factor(f32::from(options.ui_scale) / 100.0);
        }
        if options.animations != previous.animations {
            let seconds = if options.animations {
                egui::Style::default().animation_time
            } else {
                0.0
            };
            ctx.style_mut(|style| style.animation_time = seconds);
        }
        if options.fullscreen != previous.fullscreen {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(options.fullscreen));
        }
        self.applied_options = Some(options);
        // The theme plays in game by default, and can be disabled in Settings.
        let wanted =
            (self.save_recovery.is_some() || options.music_in_game).then_some(Track::MainTheme);
        let gain = music_gain(options.master_volume, options.music_volume, options.muted);
        self.music.set_effects_gain(music_gain(
            options.master_volume,
            options.effects_volume,
            options.muted,
        ));
        if self
            .music
            .update(wanted, gain, ctx.input(|input| input.time))
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}
