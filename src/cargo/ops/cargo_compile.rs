//!
//! Cargo compile currently does the following steps:
//!
//! All configurations are already injected as environment variables via the
//! main cargo command
//!
//! 1. Read the manifest
//! 2. Shell out to `cargo-resolve` with a list of dependencies and sources as
//!    stdin
//!
//!    a. Shell out to `--do update` and `--do list` for each source
//!    b. Resolve dependencies and return a list of name/version/source
//!
//! 3. Shell out to `--do download` for each source
//! 4. Shell out to `--do get` for each source, and build up the list of paths
//!    to pass to rustc -L
//! 5. Call `cargo-rustc` with the results of the resolver zipped together with
//!    the results of the `get`
//!
//!    a. Topologically sort the dependencies
//!    b. Compile each dependency in order, passing in the -L's pointing at each
//!       previously compiled dependency
//!

use std::os;
use std::collections::HashMap;
use std::io::File;
use serialize::Decodable;
use rstoml = toml;

use core::registry::PackageRegistry;
use core::{MultiShell, Source, SourceId, PackageSet, Target, PackageId, Resolve, resolver};
use ops;
use sources::{PathSource};
use util::config::{Config, ConfigValue};
use util::{CargoResult, Wrap, config, internal, human, ChainError, toml};

pub struct CompileOptions<'a> {
    pub update: bool,
    pub env: &'a str,
    pub shell: &'a mut MultiShell,
    pub jobs: Option<uint>,
    pub target: Option<&'a str>,
}

pub fn compile(manifest_path: &Path,
               options: &mut CompileOptions) -> CargoResult<Vec<String>> {
    let CompileOptions { update, env, ref mut shell, jobs, target } = *options;
    let target = target.map(|s| s.to_string());

    log!(4, "compile; manifest-path={}", manifest_path.display());

    let mut source = PathSource::for_path(&manifest_path.dir_path());

    try!(source.update());

    // TODO: Move this into PathSource
    let package = try!(source.get_root_package());
    debug!("loaded package; package={}", package);

    for key in package.get_manifest().get_unused_keys().iter() {
        try!(shell.warn(format!("unused manifest key: {}", key)));
    }

    let user_configs = try!(config::all_configs(os::getcwd()));
    let override_ids = try!(source_ids_from_config(&user_configs,
                                                   manifest_path.dir_path()));
    let source_ids = package.get_source_ids();

    let (packages, resolve, resolve_with_overrides, sources) = {
        let lockfile = manifest_path.dir_path().join("Cargo.lock");
        let source_id = package.get_package_id().get_source_id();

        let mut config = try!(Config::new(*shell, update, jobs, target.clone()));

        let mut registry = try!(PackageRegistry::new(source_ids, &mut config));

        let resolved = match try!(load_lockfile(&lockfile, source_id)) {
            Some(r) => r,
            None => {
                try!(resolver::resolve(package.get_package_id(),
                                       package.get_dependencies(),
                                       &mut registry))
            }
        };

        try!(registry.add_overrides(override_ids));

        let resolved_with_overrides =
                try!(resolver::resolve(package.get_package_id(),
                                       package.get_dependencies(),
                                       &mut registry));

        let req: Vec<PackageId> = resolved_with_overrides.iter().map(|r| {
            r.clone()
        }).collect();
        let packages = try!(registry.get(req.as_slice()).wrap({
            human("Unable to get packages from source")
        }));

        (packages, resolved, resolved_with_overrides, registry.move_sources())
    };

    debug!("packages={}", packages);

    let targets = package.get_targets().iter().filter(|target| {
        match env {
            // doc-all == document everything, so look for doc targets
            "doc" | "doc-all" => target.get_profile().get_env() == "doc",
            env => target.get_profile().get_env() == env,
        }
    }).collect::<Vec<&Target>>();

    {
        let mut config = try!(Config::new(*shell, update, jobs, target));
        try!(scrape_target_config(&mut config, &user_configs));

        try!(ops::compile_targets(env.as_slice(), targets.as_slice(), &package,
                                  &PackageSet::new(packages.as_slice()),
                                  &resolve_with_overrides, &sources,
                                  &mut config));
    }

    try!(ops::write_resolve(&package, &resolve));

    let test_executables: Vec<String> = targets.iter()
        .filter_map(|target| {
            if target.get_profile().is_test() {
                debug!("Run  Target: {}", target.get_name());
                Some(target.file_stem())
            } else {
                debug!("Skip Target: {}", target.get_name());
                None
            }
    }).collect();

    Ok(test_executables)
}

fn load_lockfile(path: &Path, sid: &SourceId) -> CargoResult<Option<Resolve>> {
    // If there is no lockfile, return none.
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Ok(None)
    };

    let s = try!(f.read_to_string());

    let table = rstoml::Table(try!(toml::parse(s.as_slice(), path)));
    let mut d = rstoml::Decoder::new(table);
    let v: resolver::EncodableResolve = Decodable::decode(&mut d).unwrap();
    Ok(Some(try!(v.to_resolve(sid))))
}

fn source_ids_from_config(configs: &HashMap<String, config::ConfigValue>,
                          cur_path: Path) -> CargoResult<Vec<SourceId>> {
    debug!("loaded config; configs={}", configs);

    let config_paths = configs.find_equiv(&"paths").map(|v| v.clone());
    let config_paths = config_paths.unwrap_or_else(|| ConfigValue::new());

    let paths = try!(config_paths.list().chain_error(|| {
        internal("invalid configuration for the key `path`")
    }));

    // Make sure we don't override the local package, even if it's in the list
    // of override paths
    Ok(paths.iter().filter(|p| {
        cur_path != os::make_absolute(&Path::new(p.as_slice()))
    }).map(|p| {
        SourceId::for_path(&Path::new(p.as_slice()))
    }).collect())
}

fn scrape_target_config(config: &mut Config,
                        configs: &HashMap<String, config::ConfigValue>)
                        -> CargoResult<()> {
    let target = match configs.find_equiv(&"target") {
        None => return Ok(()),
        Some(target) => try!(target.table().chain_error(|| {
            internal("invalid configuration for the key `target`")
        })),
    };
    let target = match config.target() {
        None => target,
        Some(triple) => match target.find_equiv(&triple) {
            None => return Ok(()),
            Some(target) => try!(target.table().chain_error(|| {
                internal(format!("invalid configuration for the key \
                                  `target.{}`", triple))
            })),
        },
    };

    match target.find_equiv(&"ar") {
        None => {}
        Some(ar) => {
            config.set_ar(try!(ar.string().chain_error(|| {
                internal("invalid configuration for key `ar`")
            })).to_string());
        }
    }

    match target.find_equiv(&"linker") {
        None => {}
        Some(linker) => {
            config.set_linker(try!(linker.string().chain_error(|| {
                internal("invalid configuration for key `ar`")
            })).to_string());
        }
    }

    Ok(())
}
