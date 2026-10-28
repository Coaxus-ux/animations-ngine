use std::fs;

use super::FileWatch;

#[test]
fn reports_the_first_look_and_every_change() {
    let path = std::env::temp_dir().join(format!("pxe-file-watch-{}.kdl", std::process::id()));
    fs::write(&path, "a").unwrap();
    let mut watch = FileWatch::new(&path);
    assert!(watch.changed());
    assert!(!watch.changed());
    fs::write(&path, "ab").unwrap();
    assert!(watch.changed());
    fs::remove_file(&path).unwrap();
    assert!(!watch.changed());
}
