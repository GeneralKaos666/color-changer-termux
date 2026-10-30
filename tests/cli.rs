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

#[test]
fn help_wins_over_other_flags() {
    let args = Args::parse_from(["--version".into(), "--bogus".into(), "--help".into()]).unwrap();
    assert!(args.help);
    assert!(!args.version);
}

#[test]
fn short_help_alias_is_accepted() {
    assert!(Args::parse_from(["-h".into()]).unwrap().help);
}

#[test]
fn help_text_names_slots_version_and_exit_codes() {
    assert!(termux_settings::cli::HELP.contains(env!("CARGO_PKG_VERSION")));
    assert!(termux_settings::cli::HELP.contains("bold-italic"));
    assert!(termux_settings::cli::HELP.contains("Exit codes"));
}

#[test]
fn unknown_option_is_rejected() {
    let error = Args::parse_from(["--hlep".into()]).unwrap_err();
    assert!(error.contains("unknown option: --hlep"), "{error}");
}

#[test]
fn missing_flag_values_are_rejected() {
    for flag in ["--apply-font", "--slot"] {
        let error = Args::parse_from([flag.to_owned()]).unwrap_err();
        assert!(error.contains(flag), "{error}");
        assert!(error.contains("needs a value"), "{error}");
    }
}

#[test]
fn unexpected_positional_is_rejected() {
    let error = Args::parse_from(["foo".to_owned()]).unwrap_err();
    assert!(error.contains("unexpected argument: foo"), "{error}");
}

#[test]
fn help_run_returns_success() {
    let dir = tempfile::tempdir().unwrap();
    let args = Args {
        help: true,
        ..Default::default()
    };
    assert_eq!(
        run(args, &Paths::new(dir.path())).unwrap(),
        ExitCode::SUCCESS
    );
}
