use std::io::File;

use support::{project, execs, cargo_dir};
use support::{COMPILING, RUNNING, FRESH};
use support::paths::PathExt;
use hamcrest::{assert_that};

fn setup() {
}

test!(custom_build_script_failed {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]
            build = "build.rs"
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#)
        .file("build.rs", r#"
            fn main() {
                std::os::set_exit_status(101);
            }
        "#);
    assert_that(p.cargo_process("build").arg("-v"),
                execs().with_status(101)
                       .with_stdout(format!("\
{compiling} foo v0.5.0 ({url})
{running} `rustc build.rs --crate-name build-script-build --crate-type bin [..]`
{running} `[..]build-script-build`
",
url = p.url(), compiling = COMPILING, running = RUNNING))
                       .with_stderr(format!("\
Failed to run custom build command for `foo v0.5.0 ({})`
Process didn't exit successfully: `[..]build[..]build-script-build` (status=101)",
p.url())));
})

test!(custom_build_env_vars {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [features]
            bar_feat = ["bar/foo"]

            [dependencies.bar]
            path = "bar"
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#)
        .file("bar/Cargo.toml", r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]
            build = "build.rs"

            [features]
            foo = []
        "#)
        .file("bar/src/lib.rs", r#"
            pub fn hello() {}
        "#);

    let file_content = format!(r#"
            use std::os;
            use std::io::fs::PathExtensions;
            fn main() {{
                let _target = os::getenv("TARGET").unwrap();

                let _ncpus = os::getenv("NUM_JOBS").unwrap();

                let out = os::getenv("CARGO_MANIFEST_DIR").unwrap();
                let p1 = Path::new(out);
                let p2 = os::make_absolute(&Path::new(file!()).dir_path().dir_path());
                assert!(p1 == p2, "{{}} != {{}}", p1.display(), p2.display());

                let opt = os::getenv("OPT_LEVEL").unwrap();
                assert_eq!(opt.as_slice(), "0");

                let opt = os::getenv("PROFILE").unwrap();
                assert_eq!(opt.as_slice(), "compile");

                let debug = os::getenv("DEBUG").unwrap();
                assert_eq!(debug.as_slice(), "true");

                let out = os::getenv("OUT_DIR").unwrap();
                assert!(out.as_slice().starts_with(r"{0}"));
                assert!(Path::new(out).is_dir());

                let _feat = os::getenv("CARGO_FEATURE_FOO").unwrap();
            }}
        "#,
        p.root().join("target").join("build").display());

    let p = p.file("bar/build.rs", file_content);


    assert_that(p.cargo_process("build").arg("--features").arg("bar_feat"),
                execs().with_status(0));
})

test!(custom_build_script_wrong_rustc_flags {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]
            build = "build.rs"
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#)
        .file("build.rs", r#"
            fn main() {
                println!("cargo:rustc-flags=-aaa -bbb");
            }
        "#);

    assert_that(p.cargo_process("build"),
                execs().with_status(101)
                       .with_stderr(format!("\
Only `-l` and `-L` flags are allowed in build script of `foo v0.5.0 ({})`: \
`-aaa -bbb`",
p.url())));
})

/*
test!(custom_build_script_rustc_flags {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.foo]
            path = "foo"
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#)
        .file("foo/Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]
            build = "build.rs"
        "#)
        .file("foo/src/lib.rs", r#"
        "#)
        .file("foo/build.rs", r#"
            fn main() {
                println!("cargo:rustc-flags=-l nonexistinglib -L /dummy/path1 -L /dummy/path2");
            }
        "#);

    // TODO: TEST FAILS BECAUSE OF WRONG STDOUT (but otherwise, the build works)
    assert_that(p.cargo_process("build").arg("--verbose"),
                execs().with_status(101)
                       .with_stdout(format!("\
{compiling} bar v0.5.0 ({url})
{running} `rustc {dir}{sep}src{sep}lib.rs --crate-name test --crate-type lib -g \
        -C metadata=[..] \
        -C extra-filename=-[..] \
        --out-dir {dir}{sep}target \
        --dep-info [..] \
        -L {dir}{sep}target \
        -L {dir}{sep}target{sep}deps`
",
running = RUNNING, compiling = COMPILING, sep = path::SEP,
dir = p.root().display(),
url = p.url(),
)));
})
*/

test!(links_no_build_cmd {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]
            name = "foo"
            version = "0.5.0"
            authors = []
            links = "a"
        "#)
        .file("src/lib.rs", "");

    assert_that(p.cargo_process("build"),
                execs().with_status(101)
                       .with_stderr("\
package `foo v0.5.0 (file://[..])` specifies that it links to `a` but does \
not have a custom build script
"));
})

test!(links_duplicates {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]
            name = "foo"
            version = "0.5.0"
            authors = []
            links = "a"
            build = "build.rs"

            [dependencies.a]
            path = "a"
        "#)
        .file("src/lib.rs", "")
        .file("build.rs", "")
        .file("a/Cargo.toml", r#"
            [project]
            name = "a"
            version = "0.5.0"
            authors = []
            links = "a"
            build = "build.rs"
        "#)
        .file("a/src/lib.rs", "")
        .file("a/build.rs", "");

    assert_that(p.cargo_process("build"),
                execs().with_status(101)
                       .with_stderr("\
native library `a` is being linked to by more than one package, and can only be \
linked to by one package

  foo v0.5.0 (file://[..])
  a v0.5.0 (file://[..])
"));
})

test!(overrides_and_links {
    let (_, target) = ::cargo::ops::rustc_version().unwrap();

    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]
            name = "foo"
            version = "0.5.0"
            authors = []
            build = "build.rs"

            [dependencies.a]
            path = "a"
        "#)
        .file("src/lib.rs", "")
        .file("build.rs", r#"
            use std::os;
            fn main() {
                assert_eq!(os::getenv("DEP_FOO_FOO").unwrap().as_slice(), "bar");
                assert_eq!(os::getenv("DEP_FOO_BAR").unwrap().as_slice(), "baz");
            }
        "#)
        .file(".cargo/config", format!(r#"
            [target.{}.foo]
            rustc-flags = "-L foo -L bar"
            foo = "bar"
            bar = "baz"
        "#, target).as_slice())
        .file("a/Cargo.toml", r#"
            [project]
            name = "a"
            version = "0.5.0"
            authors = []
            links = "foo"
            build = "build.rs"
        "#)
        .file("a/src/lib.rs", "")
        .file("a/build.rs", "not valid rust code");

    assert_that(p.cargo_process("build").arg("-v"),
                execs().with_status(0)
                       .with_stdout(format!("\
{compiling} foo v0.5.0 (file://[..])
{running} `rustc build.rs [..]`
{compiling} a v0.5.0 (file://[..])
{running} `rustc [..] --crate-name a [..]`
{running} `[..]build-script-build`
{running} `rustc [..] --crate-name foo [..] -L foo -L bar[..]`
", compiling = COMPILING, running = RUNNING).as_slice()));
})

test!(links_passes_env_vars {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]
            name = "foo"
            version = "0.5.0"
            authors = []
            build = "build.rs"

            [dependencies.a]
            path = "a"
        "#)
        .file("src/lib.rs", "")
        .file("build.rs", r#"
            use std::os;
            fn main() {
                assert_eq!(os::getenv("DEP_FOO_FOO").unwrap().as_slice(), "bar");
                assert_eq!(os::getenv("DEP_FOO_BAR").unwrap().as_slice(), "baz");
            }
        "#)
        .file("a/Cargo.toml", r#"
            [project]
            name = "a"
            version = "0.5.0"
            authors = []
            links = "foo"
            build = "build.rs"
        "#)
        .file("a/src/lib.rs", "")
        .file("a/build.rs", r#"
            fn main() {
                println!("cargo:foo=bar");
                println!("cargo:bar=baz");
            }
        "#);

    assert_that(p.cargo_process("build").arg("-v"),
                execs().with_status(0)
                       .with_stdout(format!("\
{compiling} [..] v0.5.0 (file://[..])
{running} `rustc build.rs [..]`
{compiling} [..] v0.5.0 (file://[..])
{running} `rustc build.rs [..]`
{running} `[..]`
{running} `[..]`
{running} `[..]`
{running} `rustc [..] --crate-name foo [..]`
", compiling = COMPILING, running = RUNNING).as_slice()));
})

test!(only_rerun_build_script {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]
            name = "foo"
            version = "0.5.0"
            authors = []
            build = "build.rs"
        "#)
        .file("src/lib.rs", "")
        .file("build.rs", r#"
            fn main() {}
        "#);

    assert_that(p.cargo_process("build").arg("-v"),
                execs().with_status(0));
    p.root().move_into_the_past().unwrap();

    File::create(&p.root().join("some-new-file")).unwrap();

    assert_that(p.process(cargo_dir().join("cargo")).arg("build").arg("-v"),
                execs().with_status(0)
                       .with_stdout(format!("\
{compiling} foo v0.5.0 (file://[..])
{running} `[..]build-script-build`
{running} `rustc [..] --crate-name foo [..]`
", compiling = COMPILING, running = RUNNING).as_slice()));
})

test!(rebuild_continues_to_pass_env_vars {
    let a = project("a")
        .file("Cargo.toml", r#"
            [project]
            name = "a"
            version = "0.5.0"
            authors = []
            links = "foo"
            build = "build.rs"
        "#)
        .file("src/lib.rs", "")
        .file("build.rs", r#"
            fn main() {
                println!("cargo:foo=bar");
                println!("cargo:bar=baz");
            }
        "#);
    a.build();
    a.root().move_into_the_past().unwrap();

    let p = project("foo")
        .file("Cargo.toml", format!(r#"
            [project]
            name = "foo"
            version = "0.5.0"
            authors = []
            build = "build.rs"

            [dependencies.a]
            path = '{}'
        "#, a.root().display()))
        .file("src/lib.rs", "")
        .file("build.rs", r#"
            use std::os;
            fn main() {
                assert_eq!(os::getenv("DEP_FOO_FOO").unwrap().as_slice(), "bar");
                assert_eq!(os::getenv("DEP_FOO_BAR").unwrap().as_slice(), "baz");
            }
        "#);

    assert_that(p.cargo_process("build").arg("-v"),
                execs().with_status(0));
    p.root().move_into_the_past().unwrap();

    File::create(&p.root().join("some-new-file")).unwrap();

    assert_that(p.process(cargo_dir().join("cargo")).arg("build").arg("-v"),
                execs().with_status(0)
                       .with_stdout(format!("\
{fresh} a v0.5.0 (file://[..])
{compiling} foo v0.5.0 (file://[..])
{running} `[..]build-script-build`
{running} `rustc [..] --crate-name foo [..]`
", compiling = COMPILING, running = RUNNING, fresh = FRESH).as_slice()));
})

