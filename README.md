<p align="center"><img src="docs/logo.svg" width="96" alt="" /></p>

# routeshadow

routeshadow is an offline command-line tool for inspecting an HTTP route table. It is for people who maintain routes in a web framework, gateway, or proxy configuration and want to see which route answers a request, which route is unreachable behind an earlier route, where routes cross, or how a route can be presented as a list, regular expression, or tree.

[![CI](https://github.com/Gulliby-development/routeshadow/actions/workflows/ci.yml/badge.svg)](https://github.com/Gulliby-development/routeshadow/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

The first route in file order that accepts a request is the route that answers it. routeshadow uses only the Rust standard library: it makes no network requests and reads only files named in its arguments (or standard input when the file argument is `-`). It does not percent-decode paths. Methods, names, and path text are case-sensitive.

## Why

When a route table grows, two routes can quietly fight over the same request and a later route can become dead code. routeshadow reads the table offline and tells you which route answers a request, which routes are shadowed by an earlier one, and where two routes cross, so you find the problem before a request does.

## Install

Use a Rust toolchain with Edition 2024 support. Clone the repository and install the local application with:

```sh
git clone https://github.com/Gulliby-development/routeshadow
cd routeshadow
cargo install --path .
```

This installs from the checkout; it does not depend on or claim a published crate. Cargo places the installed executable in its bin directory: `%USERPROFILE%\.cargo\bin\routeshadow.exe` on Windows and `~/.cargo/bin/routeshadow` on macOS and Linux.

To build a release binary without installing it, run `cargo build --release`. The executable is `target\release\routeshadow.exe` on Windows and `target/release/routeshadow` on macOS and Linux.

## Usage

Run `routeshadow --help` to print the command list:

```text
usage: routeshadow check FILE
       routeshadow list FILE [--by-name]
       routeshadow match FILE METHOD PATH [--all]
       routeshadow shadow FILE
       routeshadow overlap FILE
       routeshadow url FILE ROUTE [KEY=VALUE...]
       routeshadow regex FILE [--named]
       routeshadow tree FILE
```

A route file has one route per nonblank, noncomment line. Each route line contains a method list, a path pattern, and a unique route name, separated by spaces. A line beginning with `#` (after optional spaces) is a comment. Use `*` for all methods or a comma-separated list such as `GET,POST`.

Patterns start with `/` and are split into path segments. Literal segments can be combined with typed parameters: `{id:int}` accepts digits, `{name:slug}` accepts lowercase letters and digits separated by single hyphens, and `{value}` or `{value:any}` accepts any nonempty segment. A final rest parameter such as `{path*}` accepts one or more nonempty segments. A parameter name appears at most once in a pattern. The root pattern is `/`.

Here is the complete example route file. Save it as `app.txt` yourself before running the example commands; routeshadow does not create example files.

```text
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
```

Pass a path as written, without percent-decoding or query-string processing: `?` and `#` are ordinary path characters. To read a route file from standard input, use `-` for `FILE`; otherwise only the explicitly named file is read.

For example, the tree command shows the routes in deterministic segment order:

```console
routeshadow tree app.txt
```

```text
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
```

The match command selects the first accepting route in file order. With `--all`, it also shows later matching routes:

```console
routeshadow match app.txt GET /users/42 --all
```

```text
route       users_show
line        5
pattern     /users/{id:int}
param       id=42
then        users_by_name (line 6)
```

The shadow command lists routes that can never be reached (exit code 1 when it finds one):

```console
routeshadow shadow app.txt
```

```text
files_readme (line 8) is shadowed by files_get (line 7)
health_get (line 10) is shadowed by health (line 9)
```

The overlap command lists pairs of routes that cross, with a shortest request that both accept:

```console
routeshadow overlap app.txt
```

```text
users_by_name (line 6) and latest (line 11) both match GET /users/latest
files_get (line 7) and latest (line 11) both match GET /files/latest
```

The url command builds the path of a named route, and `check` prints a summary:

```console
routeshadow url app.txt users_show id=42
routeshadow check app.txt
```

```text
/users/42
ok, 10 routes
static      6
dynamic     4
rest        1
any-method  1
deepest     2
```

## Commands

- `routeshadow check FILE` parses the file and reports route counts and summary statistics. A parser finding is printed as a finding; file and other command errors are errors.
- `routeshadow list FILE [--by-name]` prints the route table in file order, or sorts names bytewise with `--by-name`.
- `routeshadow match FILE METHOD PATH [--all]` reports the first route that accepts the method and path, with parameter values in pattern order. `--all` adds the later matches. A request with no matching route is a silent negative result.
- `routeshadow shadow FILE` reports routes wholly covered by an earlier individual route, naming the earliest route that contains each one. It does not report coverage formed only by a union of routes.
- `routeshadow overlap FILE` reports crossing route pairs: routes with a shared method and path that neither route contains. A more specific route before a general route is not a crossing.
- `routeshadow url FILE ROUTE [KEY=VALUE...]` builds a route's path from parameter assignments. Values must satisfy the parameter type; rest values use nonempty path segments separated by single slashes. Assignments are consumed in order and split at their first `=`.
- `routeshadow regex FILE [--named]` prints an anchored regular expression for each route. Literals are preserved with dots escaped, and `--named` gives parameter captures their route parameter names.
- `routeshadow tree FILE` prints a deterministic tree, merging parameter nodes by type rather than parameter name. Literal children are bytewise ordered, followed by `:int`, `:slug`, `:any`, and rest nodes.

Every command accepts a file path or `-` as its `FILE` argument. Options are `--by-name`, `--all`, and `--named` only on the commands shown above. The first route that matches wins, as in most routers.

## Exit codes

- `0`: the command completed successfully.
- `1`: a finding or negative answer. This includes a `check` parser finding and a `match` with no accepting route; both are reported without treating the input as a command error. A `shadow` or `overlap` result with findings also exits 1, while no relation findings are silent and exit 0.
- `2`: an error, such as invalid arguments, an unreadable file, invalid UTF-8, or a route-file error reported by a command other than `check`. Errors are written to standard error.

A `match` with no route and `shadow` or `overlap` with no findings produce no output. The `check` command writes parser findings and its problem count as normal command output; I/O and empty-input errors are errors instead.

## Development

Run the project checks from the repository root:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

The project is dependency-free and is intended to build on Windows, macOS, and Linux.

## License

MIT, see [LICENSE](LICENSE).

---

Built with BAKED
