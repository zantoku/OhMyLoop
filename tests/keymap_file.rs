use ohmyloop::keymap;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn keymap_reads_are_bounded_and_never_overwrite_user_configuration() {
    let directory = std::env::temp_dir().join(format!(
        "ohmyloop-keymap-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("keymap.json");
    let missing = keymap::read(&path);
    assert!(missing.configuration.is_none() && missing.warning.is_none());
    assert!(!path.exists());
    let valid = br#"{"version":1,"keys":{"jump_loop_start":["q"]}}"#;
    fs::write(&path, valid).unwrap();
    let loaded = keymap::read(&path);
    assert_eq!(
        loaded.configuration.unwrap()["keys"]["jump_loop_start"][0],
        "q"
    );
    assert!(loaded.warning.is_none());
    for invalid in [
        b"{broken".as_slice(),
        b"null".as_slice(),
        &vec![b' '; 65_537],
    ] {
        fs::write(&path, invalid).unwrap();
        let loaded = keymap::read(&path);
        assert!(loaded.warning.is_some() && loaded.configuration.is_none());
        assert_eq!(fs::read(&path).unwrap(), invalid);
    }
    fs::remove_dir_all(directory).unwrap();
}
