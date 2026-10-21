const README: &str = include_str!("../README.md");

#[test]
fn documents_required_headings() {
    assert!(README.contains("## Install"));
    assert!(README.contains("## Usage"));
    assert!(README.contains("## Commands"));
    assert!(README.contains("## Exit codes"));
    assert!(README.contains("## Development"));
}

#[test]
fn documents_exact_help_usage_lines() {
    assert!(README.contains(
        "usage: routeshadow check FILE\n       routeshadow list FILE [--by-name]\n       routeshadow match FILE METHOD PATH [--all]\n       routeshadow shadow FILE\n       routeshadow overlap FILE\n       routeshadow url FILE ROUTE [KEY=VALUE...]\n       routeshadow regex FILE [--named]\n       routeshadow tree FILE"
    ));
}

#[test]
fn documents_complete_app_fixture() {
    assert!(README.contains(
        r#"```text
# a small web app
GET / home
GET,POST /users users
GET /users/me users_me
GET /users/{id:int} users_show
GET /users/{name} users_by_name
GET /files/{path*} files_get
GET /files/readme.txt files_readme
* /health health
GET /health health_get
GET /{kind}/latest latest
```"#
    ));
}

#[test]
fn documents_exact_tree_output() {
    assert!(README.contains(
        r#"```text
/ = home (GET)
  files
    readme.txt = files_readme (GET)
    ** = files_get (GET)
  health = health (*), health_get (GET)
  users = users (GET,POST)
    me = users_me (GET)
    :int = users_show (GET)
    :any = users_by_name (GET)
  :any
    latest = latest (GET)
```"#
    ));
}

#[test]
fn documents_exact_match_all_output() {
    assert!(README.contains(
        r#"```text
route       users_show
line        5
pattern     /users/{id:int}
param       id=42
then        users_by_name (line 6)
```"#
    ));
}

#[test]
fn documents_install_and_development_guidance() {
    assert!(README.contains("Edition 2024"));
    assert!(README.contains("cargo install --path ."));
    assert!(README.contains("target\\release\\routeshadow.exe"));
    assert!(README.contains("target/release/routeshadow"));
    assert!(README.contains("cargo fmt --check"));
    assert!(README.contains("cargo clippy -- -D warnings"));
    assert!(README.contains("cargo test"));
}

#[test]
fn documents_exit_code_guidance() {
    assert!(README.contains("`0`:"));
    assert!(README.contains("`1`:"));
    assert!(README.contains("`2`:"));
    assert!(README.contains("A `match` with no route"));
    assert!(README.contains("parser findings and its problem count"));
}
