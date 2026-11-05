use super::*;

#[test]
fn preferences_remember_the_selected_save_and_controls() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    assert_eq!(Preferences::load(&path).unwrap(), Preferences::default());
    let settings = Preferences {
        slot: 2,
        save_path: "data/slot-2.json".into(),
        speed: 4,
        guide: false,
        autosave: false,
        ..Preferences::default()
    };
    settings.save(&path).unwrap();
    assert_eq!(Preferences::load(&path).unwrap(), settings);
}

#[test]
fn invalid_preferences_fail_without_overwriting_the_last_good_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    Preferences::default().save(&path).unwrap();
    let original = std::fs::read(&path).unwrap();
    let invalid = Preferences {
        speed: 0,
        ..Preferences::default()
    };
    assert!(invalid.save(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let collision = Preferences {
        save_path: "data/SETTINGS.json".into(),
        ..Preferences::default()
    };
    assert!(collision.save(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::write(&path, "{ broken }").unwrap();
    assert!(Preferences::load(&path).is_err());
}
