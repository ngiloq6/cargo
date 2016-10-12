use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use semver::Version;

use core::{PackageId, Package, Target};
use util::{self, CargoResult, Config, ProcessBuilder, process};

/// A structure returning the result of a compilation.
pub struct Compilation<'cfg> {
    /// All libraries which were built for a package.
    ///
    /// This is currently used for passing --extern flags to rustdoc tests later
    /// on.
    pub libraries: HashMap<PackageId, Vec<(Target, PathBuf)>>,

    /// An array of all tests created during this compilation.
    pub tests: Vec<(Package, String, PathBuf)>,

    /// An array of all binaries created.
    pub binaries: Vec<PathBuf>,

    /// All directires for the output of native build commands.
    ///
    /// This is currently used to drive some entries which are added to the
    /// LD_LIBRARY_PATH as appropriate.
    // TODO: deprecated, remove
    pub native_dirs: HashSet<PathBuf>,

    /// Root output directory (for the local package's artifacts)
    pub root_output: PathBuf,

    /// Output directory for rust dependencies
    pub deps_output: PathBuf,

    /// Extra environment variables that were passed to compilations and should
    /// be passed to future invocations of programs.
    pub extra_env: HashMap<PackageId, Vec<(String, String)>>,

    pub to_doc_test: Vec<Package>,

    /// Features enabled during this compilation.
    pub cfgs: HashSet<String>,

    pub target: String,

    config: &'cfg Config,
}

#[derive(Clone, Debug)]
pub enum CommandType {
    Rustc,
    Rustdoc,

    /// The command is to be executed for the target architecture.
    Target(OsString),

    /// The command is to be executed for the host architecture.
    Host(OsString),
}

impl<'cfg> Compilation<'cfg> {
    pub fn new(config: &'cfg Config) -> Compilation<'cfg> {
        Compilation {
            libraries: HashMap::new(),
            native_dirs: HashSet::new(),  // TODO: deprecated, remove
            root_output: PathBuf::from("/"),
            deps_output: PathBuf::from("/"),
            tests: Vec::new(),
            binaries: Vec::new(),
            extra_env: HashMap::new(),
            to_doc_test: Vec::new(),
            cfgs: HashSet::new(),
            config: config,
            target: String::new(),
        }
    }

    /// See `process`.
    pub fn rustc_process(&self, pkg: &Package) -> CargoResult<ProcessBuilder> {
        self.process(CommandType::Rustc, pkg)
    }

    /// See `process`.
    pub fn rustdoc_process(&self, pkg: &Package)
                           -> CargoResult<ProcessBuilder> {
        self.process(CommandType::Rustdoc, pkg)
    }

    /// See `process`.
    pub fn target_process<T: AsRef<OsStr>>(&self, cmd: T, pkg: &Package)
                                           -> CargoResult<ProcessBuilder> {
        self.process(CommandType::Target(cmd.as_ref().to_os_string()), pkg)
    }

    /// See `process`.
    pub fn host_process<T: AsRef<OsStr>>(&self, cmd: T, pkg: &Package)
                                         -> CargoResult<ProcessBuilder> {
        self.process(CommandType::Host(cmd.as_ref().to_os_string()), pkg)
    }

    /// Prepares a new process with an appropriate environment to run against
    /// the artifacts produced by the build process.
    ///
    /// The package argument is also used to configure environment variables as
    /// well as the working directory of the child process.
    pub fn process(&self, cmd: CommandType, pkg: &Package)
                   -> CargoResult<ProcessBuilder> {
        let mut search_path = vec![];

        // Add -L arguments, after stripping off prefixes like "native=" or "framework=".
        for dir in self.native_dirs.iter() {
            let dir = match dir.to_str() {
                Some(s) => {
                    let mut parts = s.splitn(2, '=');
                    match (parts.next(), parts.next()) {
                        (Some("native"), Some(path)) |
                        (Some("crate"), Some(path)) |
                        (Some("dependency"), Some(path)) |
                        (Some("framework"), Some(path)) |
                        (Some("all"), Some(path)) => path.into(),
                        _ => dir.clone(),
                    }
                }
                None => dir.clone(),
            };
            search_path.push(dir);
        }
        search_path.push(self.root_output.clone());
        search_path.push(self.deps_output.clone());
        search_path.extend(util::dylib_path().into_iter());
        let search_path = try!(util::join_paths(&search_path,
                                                util::dylib_path_envvar()));
        let mut cmd = match cmd {
            CommandType::Rustc => try!(self.config.rustc()).process(),
            CommandType::Rustdoc => process(&*try!(self.config.rustdoc())),
            CommandType::Target(ref s) |
            CommandType::Host(ref s) => process(s),
        };
        cmd.env(util::dylib_path_envvar(), &search_path);
        if let Some(env) = self.extra_env.get(pkg.package_id()) {
            for &(ref k, ref v) in env {
                cmd.env(k, v);
            }
        }

        let metadata = pkg.manifest().metadata();

        cmd.env("CARGO_MANIFEST_DIR", pkg.root())
           .env("CARGO_PKG_VERSION_MAJOR", &pkg.version().major.to_string())
           .env("CARGO_PKG_VERSION_MINOR", &pkg.version().minor.to_string())
           .env("CARGO_PKG_VERSION_PATCH", &pkg.version().patch.to_string())
           .env("CARGO_PKG_VERSION_PRE", &pre_version_component(pkg.version()))
           .env("CARGO_PKG_VERSION", &pkg.version().to_string())
           .env("CARGO_PKG_NAME", &pkg.name())
           .env("CARGO_PKG_DESCRIPTION", metadata.description.as_ref().unwrap_or(&String::new()))
           .env("CARGO_PKG_HOMEPAGE", metadata.homepage.as_ref().unwrap_or(&String::new()))
           .env("CARGO_PKG_AUTHORS", &pkg.authors().join(":"))
           .cwd(pkg.root());
        Ok(cmd)
    }
}

fn pre_version_component(v: &Version) -> String {
    if v.pre.is_empty() {
        return String::new();
    }

    let mut ret = String::new();

    for (i, x) in v.pre.iter().enumerate() {
        if i != 0 { ret.push('.') };
        ret.push_str(&x.to_string());
    }

    ret
}
