//! Window/taskbar icon from the Blender-rendered app icon (tools/art/build_app_icon.py).
//! winit applies it on Windows and X11. macOS ignores window icons; its Dock icon
//! comes from the .app bundle built by tools/package_macos.sh.

const PNG: &[u8] = include_bytes!("../../../assets/app-icon/app-icon-256.png");

pub fn window_icon() -> Option<winit::window::Icon> {
    let rgba = image::load_from_memory(PNG).ok()?.into_rgba8();
    let (width, height) = rgba.dimensions();
    winit::window::Icon::from_rgba(rgba.into_raw(), width, height).ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_app_icon_decodes_to_a_square_window_icon() {
        let rgba = image::load_from_memory(super::PNG).unwrap().into_rgba8();
        assert_eq!(rgba.dimensions(), (256, 256));
        assert!(super::window_icon().is_some());
    }
}
