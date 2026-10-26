use std::os::unix::fs::PermissionsExt;
use std::process::ExitCode;

use termux_settings::cli::{Args, run};
use termux_settings::paths::Paths;

#[test]
fn list_fonts_reports_unreadable_library() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    let fonts = dir.path().join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::set_permissions(&fonts, std::fs::Permissions::from_mode(0o000)).unwrap();

    let args = Args {
        list_fonts: true,
        ..Default::default()
    };
    assert!(run(args, &paths).is_err());

    std::fs::set_permissions(&fonts, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn apply_font_unknown_name_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    let args = Args {
        apply_font: Some("nope".to_owned()),
        ..Default::default()
    };
    assert_eq!(run(args, &paths).unwrap(), ExitCode::from(2));
}

#[test]
fn unknown_slot_is_rejected() {
    let args = Args::parse_from(["--slot".to_owned(), "bogus".to_owned()]);
    assert!(args.is_err());
}
