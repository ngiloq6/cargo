//! Tests for the `cargo read-manifest` command.

use cargo_test_support::{basic_bin_manifest, basic_bin_manifest_with_readme, main_file, project};

fn manifest_output(readme_value: &str) -> String {
    format!(
        r#"
{{
    "authors": [
        "wycats@example.com"
    ],
    "categories": [],
    "name":"foo",
    "readme": {},
    "repository": null,
    "version":"0.5.0",
    "id":"foo[..]0.5.0[..](path+file://[..]/foo)",
    "keywords": [],
    "license": null,
    "license_file": null,
    "links": null,
    "description": null,
    "edition": "2015",
    "source":null,
    "dependencies":[],
    "targets":[{{
        "kind":["bin"],
        "crate_types":["bin"],
        "doctest": false,
        "edition": "2015",
        "name":"foo",
        "src_path":"[..]/foo/src/foo.rs"
    }}],
    "features":{{}},
    "manifest_path":"[..]Cargo.toml",
    "metadata": null,
    "publish": null
}}"#,
        readme_value
    )
}

fn manifest_output_no_readme() -> String {
    manifest_output("null")
}

#[cargo_test]
fn cargo_read_manifest_path_to_cargo_toml_relative() {
    let p = project()
        .file("Cargo.toml", &basic_bin_manifest("foo"))
        .file("src/foo.rs", &main_file(r#""i am foo""#, &[]))
        .build();

    p.cargo("read-manifest --manifest-path foo/Cargo.toml")
        .cwd(p.root().parent().unwrap())
        .with_json(&manifest_output_no_readme())
        .run();
}

#[cargo_test]
fn cargo_read_manifest_path_to_cargo_toml_absolute() {
    let p = project()
        .file("Cargo.toml", &basic_bin_manifest("foo"))
        .file("src/foo.rs", &main_file(r#""i am foo""#, &[]))
        .build();

    p.cargo("read-manifest --manifest-path")
        .arg(p.root().join("Cargo.toml"))
        .cwd(p.root().parent().unwrap())
        .with_json(&manifest_output_no_readme())
        .run();
}

#[cargo_test]
fn cargo_read_manifest_path_to_cargo_toml_parent_relative() {
    let p = project()
        .file("Cargo.toml", &basic_bin_manifest("foo"))
        .file("src/foo.rs", &main_file(r#""i am foo""#, &[]))
        .build();

    p.cargo("read-manifest --manifest-path foo")
        .cwd(p.root().parent().unwrap())
        .with_status(101)
        .with_stderr(
            "[ERROR] the manifest-path must be \
             a path to a Cargo.toml file",
        )
        .run();
}

#[cargo_test]
fn cargo_read_manifest_path_to_cargo_toml_parent_absolute() {
    let p = project()
        .file("Cargo.toml", &basic_bin_manifest("foo"))
        .file("src/foo.rs", &main_file(r#""i am foo""#, &[]))
        .build();

    p.cargo("read-manifest --manifest-path")
        .arg(p.root())
        .cwd(p.root().parent().unwrap())
        .with_status(101)
        .with_stderr(
            "[ERROR] the manifest-path must be \
             a path to a Cargo.toml file",
        )
        .run();
}

#[cargo_test]
fn cargo_read_manifest_cwd() {
    let p = project()
        .file("Cargo.toml", &basic_bin_manifest("foo"))
        .file("src/foo.rs", &main_file(r#""i am foo""#, &[]))
        .build();

    p.cargo("read-manifest")
        .with_json(&manifest_output_no_readme())
        .run();
}

#[cargo_test]
fn cargo_read_manifest_default_readme() {
    let readme_filenames = ["README.md", "README.txt", "README"];

    for readme in readme_filenames.iter() {
        let p = project()
            .file("Cargo.toml", &basic_bin_manifest("foo"))
            .file(readme, "Sample project")
            .file("src/foo.rs", &main_file(r#""i am foo""#, &[]))
            .build();

        p.cargo("read-manifest")
            .with_json(&manifest_output(&format!(r#""{}""#, readme)))
            .run();
    }
}

#[cargo_test]
fn cargo_read_manifest_suppress_default_readme() {
    let p = project()
        .file(
            "Cargo.toml",
            &basic_bin_manifest_with_readme("foo", "false"),
        )
        .file("README.txt", "Sample project")
        .file("src/foo.rs", &main_file(r#""i am foo""#, &[]))
        .build();

    p.cargo("read-manifest")
        .with_json(&manifest_output_no_readme())
        .run();
}
