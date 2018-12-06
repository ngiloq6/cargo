use serde_json;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::prelude::*;
use std::str;

use crate::support::cargo_process;
use crate::support::git;
use crate::support::paths;
use crate::support::registry::{cksum, Package};
use crate::support::{basic_manifest, project, ProjectBuilder};

fn setup() {
    let root = paths::root();
    t!(fs::create_dir(&root.join(".cargo")));
    t!(t!(File::create(root.join(".cargo/config"))).write_all(
        br#"
            [source.crates-io]
            replace-with = 'my-awesome-local-registry'

            [source.my-awesome-local-registry]
            directory = 'index'
        "#
    ));
}

struct VendorPackage {
    p: Option<ProjectBuilder>,
    cksum: Checksum,
}

#[derive(Serialize)]
struct Checksum {
    package: Option<String>,
    files: HashMap<String, String>,
}

impl VendorPackage {
    fn new(name: &str) -> VendorPackage {
        VendorPackage {
            p: Some(project().at(&format!("index/{}", name))),
            cksum: Checksum {
                package: Some(String::new()),
                files: HashMap::new(),
            },
        }
    }

    fn file(&mut self, name: &str, contents: &str) -> &mut VendorPackage {
        self.p = Some(self.p.take().unwrap().file(name, contents));
        self.cksum
            .files
            .insert(name.to_string(), cksum(contents.as_bytes()));
        self
    }

    fn disable_checksum(&mut self) -> &mut VendorPackage {
        self.cksum.package = None;
        self
    }

    fn no_manifest(mut self) -> Self {
        self.p = self.p.map(|pb| pb.no_manifest());
        self
    }

    fn build(&mut self) {
        let p = self.p.take().unwrap();
        let json = serde_json::to_string(&self.cksum).unwrap();
        let p = p.file(".cargo-checksum.json", &json);
        let _ = p.build();
    }
}

#[test]
fn simple() {
    setup();

    VendorPackage::new("bar")
        .file("Cargo.toml", &basic_manifest("bar", "0.1.0"))
        .file("src/lib.rs", "pub fn bar() {}")
        .build();

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file(
            "src/lib.rs",
            "extern crate bar; pub fn foo() { bar::bar(); }",
        ).build();

    p.cargo("build")
        .with_stderr(
            "\
[COMPILING] bar v0.1.0
[COMPILING] foo v0.1.0 ([CWD])
[FINISHED] [..]
",
        ).run();
}

#[test]
fn simple_install() {
    setup();

    VendorPackage::new("foo")
        .file("src/lib.rs", "pub fn foo() {}")
        .build();

    VendorPackage::new("bar")
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "bar"
            version = "0.1.0"
            authors = []

            [dependencies]
            foo = "0.0.1"
        "#,
        ).file(
            "src/main.rs",
            "extern crate foo; pub fn main() { foo::foo(); }",
        ).build();

    cargo_process("install bar")
        .with_stderr(
            "  Installing bar v0.1.0
   Compiling foo v0.0.1
   Compiling bar v0.1.0
    Finished release [optimized] target(s) in [..]s
  Installing [..]bar[..]
warning: be sure to add `[..]` to your PATH to be able to run the installed binaries
",
        ).run();
}

#[test]
fn simple_install_fail() {
    setup();

    VendorPackage::new("foo")
        .file("src/lib.rs", "pub fn foo() {}")
        .build();

    VendorPackage::new("bar")
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "bar"
            version = "0.1.0"
            authors = []

            [dependencies]
            foo = "0.1.0"
            baz = "9.8.7"
        "#,
        ).file(
            "src/main.rs",
            "extern crate foo; pub fn main() { foo::foo(); }",
        ).build();

    cargo_process("install bar")
        .with_status(101)
        .with_stderr(
            "  Installing bar v0.1.0
error: failed to compile `bar v0.1.0`, intermediate artifacts can be found at `[..]`

Caused by:
  no matching package named `baz` found
location searched: registry `https://github.com/rust-lang/crates.io-index`
did you mean: bar, foo
required by package `bar v0.1.0`
",
        ).run();
}

#[test]
fn install_without_feature_dep() {
    setup();

    VendorPackage::new("foo")
        .file("src/lib.rs", "pub fn foo() {}")
        .build();

    VendorPackage::new("bar")
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "bar"
            version = "0.1.0"
            authors = []

            [dependencies]
            foo = "0.0.1"
            baz = { version = "9.8.7", optional = true }

            [features]
            wantbaz = ["baz"]
        "#,
        ).file(
            "src/main.rs",
            "extern crate foo; pub fn main() { foo::foo(); }",
        ).build();

    cargo_process("install bar")
        .with_stderr(
            "  Installing bar v0.1.0
   Compiling foo v0.0.1
   Compiling bar v0.1.0
    Finished release [optimized] target(s) in [..]s
  Installing [..]bar[..]
warning: be sure to add `[..]` to your PATH to be able to run the installed binaries
",
        ).run();
}

#[test]
fn not_there() {
    setup();

    let _ = project().at("index").build();

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file(
            "src/lib.rs",
            "extern crate bar; pub fn foo() { bar::bar(); }",
        ).build();

    p.cargo("build")
        .with_status(101)
        .with_stderr(
            "\
error: no matching package named `bar` found
location searched: [..]
required by package `foo v0.1.0 ([..])`
",
        ).run();
}

#[test]
fn multiple() {
    setup();

    VendorPackage::new("bar-0.1.0")
        .file("Cargo.toml", &basic_manifest("bar", "0.1.0"))
        .file("src/lib.rs", "pub fn bar() {}")
        .file(".cargo-checksum", "")
        .build();

    VendorPackage::new("bar-0.2.0")
        .file("Cargo.toml", &basic_manifest("bar", "0.2.0"))
        .file("src/lib.rs", "pub fn bar() {}")
        .file(".cargo-checksum", "")
        .build();

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file(
            "src/lib.rs",
            "extern crate bar; pub fn foo() { bar::bar(); }",
        ).build();

    p.cargo("build")
        .with_stderr(
            "\
[COMPILING] bar v0.1.0
[COMPILING] foo v0.1.0 ([CWD])
[FINISHED] [..]
",
        ).run();
}

#[test]
fn crates_io_then_directory() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file(
            "src/lib.rs",
            "extern crate bar; pub fn foo() { bar::bar(); }",
        ).build();

    let cksum = Package::new("bar", "0.1.0")
        .file("src/lib.rs", "pub fn bar() -> u32 { 0 }")
        .publish();

    p.cargo("build")
        .with_stderr(
            "\
[UPDATING] `[..]` index
[DOWNLOADING] crates ...
[DOWNLOADED] bar v0.1.0 ([..])
[COMPILING] bar v0.1.0
[COMPILING] foo v0.1.0 ([CWD])
[FINISHED] [..]
",
        ).run();

    setup();

    let mut v = VendorPackage::new("bar");
    v.file("Cargo.toml", &basic_manifest("bar", "0.1.0"));
    v.file("src/lib.rs", "pub fn bar() -> u32 { 1 }");
    v.cksum.package = Some(cksum);
    v.build();

    p.cargo("build")
        .with_stderr(
            "\
[COMPILING] bar v0.1.0
[COMPILING] foo v0.1.0 ([CWD])
[FINISHED] [..]
",
        ).run();
}

#[test]
fn crates_io_then_bad_checksum() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file("src/lib.rs", "")
        .build();

    Package::new("bar", "0.1.0").publish();

    p.cargo("build").run();
    setup();

    VendorPackage::new("bar")
        .file("Cargo.toml", &basic_manifest("bar", "0.1.0"))
        .file("src/lib.rs", "")
        .build();

    p.cargo("build")
        .with_status(101)
        .with_stderr(
            "\
error: checksum for `bar v0.1.0` changed between lock files

this could be indicative of a few possible errors:

    * the lock file is corrupt
    * a replacement source in use (e.g. a mirror) returned a different checksum
    * the source itself may be corrupt in one way or another

unable to verify that `bar v0.1.0` is the same as when the lockfile was generated

",
        ).run();
}

#[test]
fn bad_file_checksum() {
    setup();

    VendorPackage::new("bar")
        .file("Cargo.toml", &basic_manifest("bar", "0.1.0"))
        .file("src/lib.rs", "")
        .build();

    let mut f = t!(File::create(paths::root().join("index/bar/src/lib.rs")));
    t!(f.write_all(b"fn bar() -> u32 { 0 }"));

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file("src/lib.rs", "")
        .build();

    p.cargo("build")
        .with_status(101)
        .with_stderr(
            "\
error: the listed checksum of `[..]lib.rs` has changed:
expected: [..]
actual:   [..]

directory sources are not intended to be edited, if modifications are \
required then it is recommended that [replace] is used with a forked copy of \
the source
",
        ).run();
}

#[test]
fn only_dot_files_ok() {
    setup();

    VendorPackage::new("bar")
        .file("Cargo.toml", &basic_manifest("bar", "0.1.0"))
        .file("src/lib.rs", "")
        .build();
    VendorPackage::new("foo")
        .no_manifest()
        .file(".bar", "")
        .build();

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file("src/lib.rs", "")
        .build();

    p.cargo("build").run();
}

#[test]
fn random_files_ok() {
    setup();

    VendorPackage::new("bar")
        .file("Cargo.toml", &basic_manifest("bar", "0.1.0"))
        .file("src/lib.rs", "")
        .build();
    VendorPackage::new("foo")
        .no_manifest()
        .file("bar", "")
        .file("../test", "")
        .build();

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file("src/lib.rs", "")
        .build();

    p.cargo("build").run();
}

#[test]
fn git_lock_file_doesnt_change() {
    let git = git::new("git", |p| {
        p.file("Cargo.toml", &basic_manifest("git", "0.5.0"))
            .file("src/lib.rs", "")
    }).unwrap();

    VendorPackage::new("git")
        .file("Cargo.toml", &basic_manifest("git", "0.5.0"))
        .file("src/lib.rs", "")
        .disable_checksum()
        .build();

    let p = project()
        .file(
            "Cargo.toml",
            &format!(
                r#"
            [package]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies]
            git = {{ git = '{0}' }}
        "#,
                git.url()
            ),
        ).file("src/lib.rs", "")
        .build();

    p.cargo("build").run();

    let mut lock1 = String::new();
    t!(t!(File::open(p.root().join("Cargo.lock"))).read_to_string(&mut lock1));

    let root = paths::root();
    t!(fs::create_dir(&root.join(".cargo")));
    t!(t!(File::create(root.join(".cargo/config"))).write_all(
        &format!(
            r#"
        [source.my-git-repo]
        git = '{}'
        replace-with = 'my-awesome-local-registry'

        [source.my-awesome-local-registry]
        directory = 'index'
    "#,
            git.url()
        ).as_bytes()
    ));

    p.cargo("build")
        .with_stderr(
            "\
[COMPILING] [..]
[COMPILING] [..]
[FINISHED] [..]
",
        ).run();

    let mut lock2 = String::new();
    t!(t!(File::open(p.root().join("Cargo.lock"))).read_to_string(&mut lock2));
    assert_eq!(lock1, lock2, "lock files changed");
}

#[test]
fn git_override_requires_lockfile() {
    VendorPackage::new("git")
        .file("Cargo.toml", &basic_manifest("git", "0.5.0"))
        .file("src/lib.rs", "")
        .disable_checksum()
        .build();

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies]
            git = { git = 'https://example.com/' }
        "#,
        ).file("src/lib.rs", "")
        .build();

    let root = paths::root();
    t!(fs::create_dir(&root.join(".cargo")));
    t!(t!(File::create(root.join(".cargo/config"))).write_all(
        br#"
        [source.my-git-repo]
        git = 'https://example.com/'
        replace-with = 'my-awesome-local-registry'

        [source.my-awesome-local-registry]
        directory = 'index'
    "#
    ));

    p.cargo("build")
        .with_status(101)
        .with_stderr(
            "\
error: failed to load source for a dependency on `git`

Caused by:
  Unable to update [..]

Caused by:
  the source my-git-repo requires a lock file to be present first before it can be
used against vendored source code

remove the source replacement configuration, generate a lock file, and then
restore the source replacement configuration to continue the build

",
        ).run();
}

#[test]
fn workspace_different_locations() {
    let p = project()
        .no_manifest()
        .file(
            "foo/Cargo.toml",
            r#"
                [package]
                name = 'foo'
                version = '0.1.0'

                [dependencies]
                baz = "*"
            "#,
        ).file("foo/src/lib.rs", "")
        .file("foo/vendor/baz/Cargo.toml", &basic_manifest("baz", "0.1.0"))
        .file("foo/vendor/baz/src/lib.rs", "")
        .file("foo/vendor/baz/.cargo-checksum.json", "{\"files\":{}}")
        .file(
            "bar/Cargo.toml",
            r#"
                [package]
                name = 'bar'
                version = '0.1.0'

                [dependencies]
                baz = "*"
            "#,
        ).file("bar/src/lib.rs", "")
        .file(
            ".cargo/config",
            r#"
                [build]
                target-dir = './target'

                [source.crates-io]
                replace-with = 'my-awesome-local-registry'

                [source.my-awesome-local-registry]
                directory = 'foo/vendor'
            "#,
        ).build();

    p.cargo("build").cwd(p.root().join("foo")).run();
    p.cargo("build")
        .cwd(p.root().join("bar"))
        .with_status(0)
        .with_stderr(
            "\
[COMPILING] bar [..]
[FINISHED] [..]
",
        ).run();
}

#[test]
fn version_missing() {
    setup();

    VendorPackage::new("foo")
        .file("src/lib.rs", "pub fn foo() {}")
        .build();

    VendorPackage::new("bar")
        .file(
            "Cargo.toml",
            r#"
                [package]
                name = "bar"
                version = "0.1.0"
                authors = []

                [dependencies]
                foo = "2"
            "#,
        )
        .file("src/main.rs", "fn main() {}")
        .build();

    cargo_process("install bar")
        .with_stderr(
            "\
[INSTALLING] bar v0.1.0
error: failed to compile [..]

Caused by:
  failed to select a version for the requirement `foo = \"^2\"`
  candidate versions found which didn't match: 0.0.1
  location searched: directory source `[..] (which is replacing registry `[..]`)
required by package `bar v0.1.0`
perhaps a crate was updated and forgotten to be re-vendored?
",
        )
        .with_status(101)
        .run();
}
