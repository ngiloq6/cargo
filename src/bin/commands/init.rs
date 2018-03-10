use command_prelude::*;

pub fn cli() -> App {
    subcommand("init")
        .about("Create a new cargo package in an existing directory")
        .arg(Arg::with_name("path").default_value("."))
        .arg_new_opts()
}
