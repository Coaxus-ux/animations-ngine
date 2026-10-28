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

#[test]
fn reports_changes_in_any_used_file() {
    let dir = std::env::temp_dir();
    let id = std::process::id();
    let (root, part) = (
        dir.join(format!("pxe-watch-root-{id}.kdl")),
        dir.join(format!("pxe-watch-part-{id}.kdl")),
    );
    fs::write(&root, "a").unwrap();
    fs::write(&part, "b").unwrap();
    let mut watch = FileWatch::new(&root);
    assert!(watch.changed());
    watch.track(&[root.clone(), part.clone()]);
    assert!(!watch.changed(), "tracking a file is not a change");
    fs::write(&part, "bc").unwrap();
    assert!(watch.changed());
    fs::remove_file(&root).unwrap();
    fs::remove_file(&part).unwrap();
}
