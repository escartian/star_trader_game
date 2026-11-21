//! The Blender-rendered app icon (tools/art/build_app_icon.py): the window and
//! taskbar icon on Windows and X11, and the game chooser's logo. macOS ignores
//! window icons, and setting the Dock icon at runtime needs unsafe AppKit calls
//! (the workspace forbids unsafe code), so on macOS the Dock icon comes from
//! the .app bundle built by tools/package_macos.sh.

const PNG: &[u8] = include_bytes!("../../../assets/app-icon/app-icon-256.png");

#[cfg(not(target_arch = "wasm32"))]
pub fn window_icon() -> Option<winit::window::Icon> {
    let rgba = image::load_from_memory(PNG).ok()?.into_rgba8();
    let (width, height) = rgba.dimensions();
    winit::window::Icon::from_rgba(rgba.into_raw(), width, height).ok()
}

/// The icon as an egui texture (the game chooser's logo), decoded once per context.
pub fn texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("app-icon-texture");
    if let Some(texture) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return Some(texture);
    }
    let rgba = image::load_from_memory(PNG).ok()?.into_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    let texture = ctx.load_texture("app-icon", pixels, egui::TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    Some(texture)
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_app_icon_decodes_to_a_square_window_icon() {
        let rgba = image::load_from_memory(super::PNG).unwrap().into_rgba8();
        assert_eq!(rgba.dimensions(), (256, 256));
        assert!(super::window_icon().is_some());
        let ctx = egui::Context::default();
        let first = super::texture(&ctx).unwrap();
        assert_eq!(first.size(), [256, 256]);
        assert_eq!(
            super::texture(&ctx).unwrap().id(),
            first.id(),
            "decoded once"
        );
    }
}
