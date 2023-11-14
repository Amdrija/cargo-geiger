//! The outer CLI parts of the `cargo-geiger` cargo plugin executable.
//! TODO: Refactor this file to only deal with command line argument processing.

#![deny(clippy::cargo)]
#![deny(clippy::doc_markdown)]
#![forbid(unsafe_code)]
#![deny(warnings)]

extern crate cargo;
extern crate colored;
extern crate petgraph;
extern crate strum;
extern crate strum_macros;

use cargo::core::shell::Shell;
use cargo::util::important_paths;
use cargo::{CliError, CliResult, Config};
use cargo_geiger::args::{Args, HELP};
use cargo_geiger::cli::{get_cargo_metadata, get_krates, get_workspace};
use cargo_geiger::format::print_config::PrintConfig;
use cargo_geiger::graph::build_graph;
use cargo_geiger::mapping::{CargoMetadataParameters, QueryResolve};
use cargo_geiger::scan::ScanParameters;
use geiger::extern_syn_visitor::ExternDefinition;
use geiger::ExternCall;
use serde::Serialize;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct ExternDefinitionPrint<'a> {
    pub package_id: &'a cargo_metadata::PackageId,
    pub extern_definition: &'a ExternDefinition,
    pub extern_calls: &'a Vec<ExternCall>,
}

fn cli_result_main(args: &Args) -> CliResult {
    if args.version {
        println!("cargo-geiger {}", VERSION.unwrap_or("unknown version"));
        return Ok(());
    }
    if args.help {
        println!("{}", HELP);
        return Ok(());
    }

    let mut config = Config::default()?;
    args.update_config(&mut config)?;

    let cargo_metadata = get_cargo_metadata(args, &config)?;
    let krates = get_krates(&cargo_metadata)?;

    let cargo_metadata_parameters = CargoMetadataParameters {
        metadata: &cargo_metadata,
        krates: &krates,
    };

    let workspace = get_workspace(&config, args.manifest_path.clone())?;

    let cargo_metadata_root_package_id = if let Some(
        cargo_metadata_root_package,
    ) = cargo_metadata.root_package()
    {
        cargo_metadata_root_package.id.clone()
    } else {
        eprintln!(
            "manifest path `{}` is a virtual manifest, but this command requires running against an actual package in this workspace",
            match args.manifest_path.clone() {
                Some(path) => path,
                None => important_paths::find_root_manifest_for_wd(config.cwd())?,
            }.as_os_str().to_str().unwrap()
        );

        return Err(CliError::code(1));
    };

    let global_rustc = config.load_global_rustc(Some(&workspace))?;

    let _ = build_graph(
        args,
        &cargo_metadata_parameters,
        &global_rustc.host,
        &global_rustc.path,
        cargo_metadata_root_package_id.clone(),
    )?;

    let _ = args.package.as_ref().map_or(
        cargo_metadata_root_package_id.clone(),
        |package_query| {
            krates
                .query_resolve(package_query)
                .map_or(cargo_metadata_root_package_id, |package_id| package_id)
        },
    );

    let print_config = PrintConfig::new(&args)?;
    let scan_parameters = ScanParameters {
        args,
        config: &config,
        print_config: &print_config,
    };
    let scan_details = cargo_geiger::scan::default::scan(
        &cargo_metadata_parameters,
        &scan_parameters,
        &workspace,
    )?;

    let mut definitions: Vec<ExternDefinitionPrint> = vec![];
    for (package_id, metrics) in
        scan_details.geiger_context.package_id_to_metrics.iter()
    {
        for (def, calls) in metrics.extern_calls.iter() {
            let print_definition = ExternDefinitionPrint {
                extern_definition: def,
                extern_calls: calls,
                package_id: package_id,
            };
            definitions.push(print_definition);
        }
    }

    println!(
        "{}",
        match serde_json::to_string(&definitions) {
            Ok(str) => str,
            Err(_) => String::from("error"),
        }
    );

    Ok(())
}

fn main() {
    let args = Args::parse_args(pico_args::Arguments::from_env()).unwrap();
    if let Err(e) = cli_result_main(&args) {
        let mut shell = Shell::new();
        cargo::exit_with_error(e, &mut shell)
    }
}
