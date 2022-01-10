#[path = "../common/config.rs"]
mod config;
mod vmm;

use config::{load_vmm_config, load_app_config};
use vmm::{run_vm, VMM};

use std::path::PathBuf;
use tempfile::TempDir;

// CLI app dependencies
extern crate clap;
use clap::{Arg, App, SubCommand, ArgMatches};

use crate::vmm::VmmCrosVm;

fn cli_init() -> ArgMatches<'static> {
    App::new("OpenVMM (OpenOS)")
        .version("1.0")
        .about("OpenOS App VM Manager")
        .subcommand(SubCommand::with_name("apply")
            .about("Create/Update app config from yaml file")
            .arg(Arg::with_name("CONFIG_FILE")
                .help("Sets the config file to load")
                .required(true)
                .index(1)))
        .subcommand(SubCommand::with_name("start")
            .about("start app")
            .setting(clap::AppSettings::TrailingVarArg)
            .setting(clap::AppSettings::AllowLeadingHyphen)
            .arg(Arg::with_name("NAME")
                .help("App to start from OPENOS_CONFIG_DIR/start")
                .required(true)
                .index(1))
            .arg(Arg::with_name("ARGS")
                .help("Program arguments to pass inside VM")
                // .allow_hyphen_values(true)
                .takes_value(true)
                .multiple(true)))
        .get_matches()
}


// #[tokio::main]
fn main() {

    let vmm_config = load_vmm_config();
    println!("Loaded VmmConfig : {:#?}", vmm_config);

    let cli_matches = cli_init();

    if let Some(matches) = cli_matches.subcommand_matches("apply") {
        todo!("Apply not implemented")
    }

    if let Some(matches) = cli_matches.subcommand_matches("start") {
        let app_name = matches.value_of("NAME").unwrap();
        let app_args = matches.values_of("ARGS").unwrap_or_default().into_iter().collect();
        println!("Starting App : {:?}", app_name);
        
        let app_config = load_app_config(app_name, &app_args, &vmm_config);
        println!("Loaded App Config : {:#?}", app_config);
        
        let temp_dir: TempDir = TempDir::new().unwrap();
        println!("Temporary directory: {:?}", temp_dir);
        
        let mut vmm = VmmCrosVm::new(vmm_config);
        match vmm.load_config(&app_config, &PathBuf::from(temp_dir.path())) {
            Ok(_) => println!("Loaded VMM config"),
            Err(e) => return println!("Failed to load VMM Config: {:?}", e),
        };

        println!("Starting App VM");
        
        match run_vm(&mut vmm) {
            Ok(_) => {
                println!("OpenVM Exited normally");
            }
            Err(e) => {
                println!("crosvm has exited with error: {}", e);
            }
        }
        match temp_dir.close() {
            Ok(_) => println!("Removed Temp Directory Successfully"),
            Err(_) => panic!("Failed to remove temp dir"),
        };
    }
}