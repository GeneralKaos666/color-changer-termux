use std::path::PathBuf;
use termux_colors::paths::Paths;

#[test]
fn accessors_join_under_root() {
    let p = Paths::new("/tmp/home/.termux");
    assert_eq!(p.colors_root(), PathBuf::from("/tmp/home/.termux/colors"));
    assert_eq!(
        p.colors_properties(),
        PathBuf::from("/tmp/home/.termux/colors.properties")
    );
    assert_eq!(
        p.termux_properties(),
        PathBuf::from("/tmp/home/.termux/termux.properties")
    );
    assert_eq!(p.backups(), PathBuf::from("/tmp/home/.termux/backups"));
}

#[test]
fn termux_home_wins_over_home() {
    let p = Paths::from_parts(
        Some(PathBuf::from("/custom")),
        Some(PathBuf::from("/home/u")),
    );
    assert_eq!(p.colors_root(), PathBuf::from("/custom/colors"));
    assert_eq!(p.root(), std::path::Path::new("/custom"));
}

#[test]
fn home_is_the_fallback() {
    let p = Paths::from_parts(None, Some(PathBuf::from("/home/u")));
    assert_eq!(
        p.colors_properties(),
        PathBuf::from("/home/u/.termux/colors.properties")
    );
}
