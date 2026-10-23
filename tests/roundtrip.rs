use termux_colors::persist::colors_io::{read_colors, write_colors};
use termux_colors::persist::theme_io::{read_app_theme, write_app_theme};

#[test]
fn colors_round_trip_matches_the_expected_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("colors.properties");
    std::fs::copy("tests/fixtures/colors_with_extras.properties", &path).unwrap();
    let colors = read_colors(&path).unwrap();
    write_colors(&path, &colors, true).unwrap();
    let actual = std::fs::read(&path).unwrap();
    let expected = std::fs::read("tests/fixtures/colors_with_extras.expected").unwrap();
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected)
    );
}

#[test]
fn termux_properties_round_trip_is_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("termux.properties");
    std::fs::copy("tests/fixtures/termux.properties", &path).unwrap();
    let theme = read_app_theme(&path).unwrap();
    write_app_theme(&path, &theme, true).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        std::fs::read("tests/fixtures/termux.expected").unwrap()
    );
}

#[test]
fn extra_keys_survives_a_setting_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("termux.properties");
    std::fs::copy("tests/fixtures/termux.properties", &path).unwrap();
    let mut theme = read_app_theme(&path).unwrap();
    theme.set("use-black-ui", "false");
    write_app_theme(&path, &theme, true).unwrap();
    let out = std::fs::read_to_string(&path).unwrap();
    let original = std::fs::read_to_string("tests/fixtures/termux.properties").unwrap();
    let extra = original
        .lines()
        .find(|l| l.starts_with("extra-keys"))
        .unwrap();
    assert!(out.lines().any(|l| l == extra));
    assert!(out.contains("use-black-ui = false"));
}
