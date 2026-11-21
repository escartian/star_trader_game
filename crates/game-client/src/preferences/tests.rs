use super::*;

#[test]
fn preferences_remember_the_selected_save_and_controls() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    assert_eq!(Preferences::load(&path).unwrap(), Preferences::default());
    let settings = Preferences {
        slot: 2,
        save_path: "data/slot-2.json".into(),
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
        slot: 0,
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

#[test]
fn legacy_speed_settings_migrate_without_reintroducing_a_wall_clock() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    std::fs::write(&path, r#"{"schema_version":1,"save_path":"data/slot-2.json","slot":2,"speed":4,"guide":false,"autosave":true}"#).unwrap();
    let migrated = Preferences::load(&path).unwrap();
    assert_eq!(migrated.slot, 2);
    assert!(!migrated.guide);
    assert_eq!(migrated.schema_version, 2);
    migrated.save(&path).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert!(value.get("speed").is_none());
}

#[test]
fn settings_from_before_options_load_with_defaults_and_bad_options_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    std::fs::write(&path, r#"{"schema_version":2,"save_path":"data/slot-1.json","slot":1,"active_ship":1,"guide":true,"autosave":true}"#).unwrap();
    assert_eq!(
        Preferences::load(&path).unwrap().options,
        ClientOptions::default()
    );
    let quiet = Preferences {
        options: ClientOptions {
            music_volume: 25,
            muted: true,
            ui_scale: 150,
            ..ClientOptions::default()
        },
        ..Preferences::default()
    };
    quiet.save(&path).unwrap();
    assert_eq!(Preferences::load(&path).unwrap(), quiet);
    for options in [
        ClientOptions {
            master_volume: 101,
            ..ClientOptions::default()
        },
        ClientOptions {
            ui_scale: 50,
            ..ClientOptions::default()
        },
    ] {
        assert!(
            Preferences {
                options,
                ..Preferences::default()
            }
            .save(&path)
            .is_err()
        );
    }
}
