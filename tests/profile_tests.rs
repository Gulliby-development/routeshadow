const MANIFEST: &str = include_str!("../Cargo.toml");
const README: &str = include_str!("../README.md");

fn release_profile_lines() -> Vec<&'static str> {
    let mut lines = MANIFEST.lines();
    lines
        .by_ref()
        .find(|line| line.trim() == "[profile.release]")
        .expect("Cargo.toml has a [profile.release] table");
    lines
        .take_while(|line| !line.trim_start().starts_with('['))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

#[test]
fn release_profile_is_exactly_the_documented_settings() {
    assert_eq!(
        release_profile_lines(),
        [
            "overflow-checks = true",
            "lto = true",
            "codegen-units = 1",
            "strip = true",
            "panic = \"abort\"",
        ]
    );
}

#[test]
fn manifest_keeps_its_package_and_has_one_profile_table() {
    assert!(MANIFEST.starts_with(
        "[package]\nname = \"routeshadow\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
    ));
    assert_eq!(MANIFEST.matches("[profile.").count(), 1);
}

#[test]
fn readme_describes_the_release_profile() {
    assert!(README.contains("The release profile in `Cargo.toml`"));
    assert!(README.contains("`overflow-checks`"));
    assert!(README.contains("`panic = \"abort\"`"));
}
