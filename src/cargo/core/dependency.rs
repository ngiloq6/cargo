use semver::Version;
use core::{VersionReq};
use util::CargoResult;

#[deriving(Eq,Clone,Show)]
pub struct Dependency {
    name: StrBuf,
    req: VersionReq
}

impl Dependency {
    pub fn new(name: &str, req: &VersionReq) -> Dependency {
        Dependency {
            name: name.to_strbuf(),
            req: req.clone()
        }
    }

    pub fn parse(name: &str, version: &str) -> CargoResult<Dependency> {
        Ok(Dependency {
            name: name.to_strbuf(),
            req: try!(VersionReq::parse(version))
        })
    }

    pub fn exact(name: &str, version: &Version) -> Dependency {
        Dependency {
            name: name.to_strbuf(),
            req: VersionReq::exact(version)
        }
    }

    pub fn get_version_req<'a>(&'a self) -> &'a VersionReq {
        &self.req
    }

    pub fn get_name<'a>(&'a self) -> &'a str {
        self.name.as_slice()
    }
}

#[deriving(Eq,Clone,Encodable)]
pub struct SerializedDependency {
    name: StrBuf,
    req: StrBuf
}

impl SerializedDependency {
    pub fn from_dependency(dep: &Dependency) -> SerializedDependency {
        SerializedDependency {
            name: dep.get_name().to_strbuf(),
            req: format_strbuf!("{}", dep.get_version_req())
        }
    }
}
