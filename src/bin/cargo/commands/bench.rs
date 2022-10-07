use crate::command_prelude::*;
use cargo::ops::{self, TestOptions};

pub fn cli() -> Command {
    subcommand("bench")
        .about("Execute all benchmarks of a local package")
        .arg_quiet()
        .arg(
            Arg::new("BENCHNAME")
                .action(ArgAction::Set)
                .help("If specified, only run benches containing this string in their names"),
        )
        .arg(
            Arg::new("args")
                .help("Arguments for the bench binary")
                .num_args(0..)
                .last(true),
        )
        .arg_targets_all(
            "Benchmark only this package's library",
            "Benchmark only the specified binary",
            "Benchmark all binaries",
            "Benchmark only the specified example",
            "Benchmark all examples",
            "Benchmark only the specified test target",
            "Benchmark all tests",
            "Benchmark only the specified bench target",
            "Benchmark all benches",
            "Benchmark all targets",
        )
        .arg(flag("no-run", "Compile, but don't run benchmarks"))
        .arg_package_spec(
            "Package to run benchmarks for",
            "Benchmark all packages in the workspace",
            "Exclude packages from the benchmark",
        )
        .arg_jobs()
        .arg_profile("Build artifacts with the specified profile")
        .arg_features()
        .arg_target_triple("Build for the target triple")
        .arg_target_dir()
        .arg_manifest_path()
        .arg_ignore_rust_version()
        .arg_message_format()
        .arg(flag(
            "no-fail-fast",
            "Run all benchmarks regardless of failure",
        ))
        .arg_unit_graph()
        .arg_timings()
        .after_help("Run `cargo help bench` for more detailed information.\n")
}

pub fn exec(config: &mut Config, args: &ArgMatches) -> CliResult {
    let ws = args.workspace(config)?;
    let mut compile_opts = args.compile_options(
        config,
        CompileMode::Bench,
        Some(&ws),
        ProfileChecking::Custom,
    )?;

    compile_opts.build_config.requested_profile =
        args.get_profile_name(config, "bench", ProfileChecking::Custom)?;

    let ops = TestOptions {
        no_run: args.flag("no-run"),
        no_fail_fast: args.flag("no-fail-fast"),
        compile_opts,
    };

    let bench_args = args.get_one::<String>("BENCHNAME").into_iter();
    let bench_args = bench_args.chain(args.get_many::<String>("args").unwrap_or_default());
    let bench_args = bench_args.map(String::as_str).collect::<Vec<_>>();

    ops::run_benches(&ws, &ops, &bench_args)
}
