#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(unused_must_use)]

use crate::commandline_args::CommandlineArgs;
use clap::Parser;
use curseforge_pack_downloader::CurseforgePackDownloader;
use log::{error, info, warn};
use std::env::set_var;
use std::ffi::OsStr;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::PathBuf;
use std::process::exit;

mod commandline_args;
mod env;

#[tokio::main]
async fn main() {
    // Record the start time of the process
    let start_time = std::time::SystemTime::now();

    // Parse command line arguments
    let args = CommandlineArgs::parse();

    // Set up the logging environment
    set_var("RUST_LOG", "info");
    env_logger::init();

    // Log starting information about the tool
    info!("Starting Curseforge Pack Downloader");
    warn!("This tool is not affiliated with CurseForge in any way, in fact we strongly dislike curseforge's bullshit!");

    // Attempt to read environment variables from the embedded env.ini file
    let env = match env::Env::new() {
        Ok(env) => env,
        Err(err) => {
            // Log an error and exit if env variables cannot be read
            error!("Unable to read environment variables from the env.ini file, make sure its filled out before building: {}", err);
            exit(1);
        }
    };

    // set the `CURSEFORGE_API_KEY` environment variable
    // this will be required by the `CurseforgePackDownloader`
    set_var("CURSEFORGE_API_KEY", &env.curseforge_api_key);

    // Create an instance of the `CurseforgePackDownloader` struct.
    let mut downloader = CurseforgePackDownloader::new();

    // Set downloader options based on input arguments
    downloader.set_validate(args.validate);
    downloader.set_parallel_downloads(args.parallel_downloads);
    downloader.set_output_directory(&args.output);

    match create_dir_all(&args.output) {
        Ok(_) => match remove_dir_all(&args.output) {
            Ok(_) => {
                info!("Removed old output directory");
            }
            Err(err) => {
                error!("Unable to remove old output directory: {}", err);
                exit(1);
            }
        },
        Err(err) => {
            error!("Invalid output directory: {}", err);
            exit(1);
        }
    };

    // Set validation size limit if provided and validation is enabled
    if let Some(validate_if_less_than_bytes) = args.validate_if_size_less_than {
        downloader.set_validate_if_size_less_than(validate_if_less_than_bytes);
    }

    // Determine processing path based on input ID or file
    match if let Some(id) = args.id {
        downloader.set_temp_directory(format!(
            "{}-{}.temp",
            id,
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::new(0, 0))
                .as_millis()
        ));
        downloader.process_id(id, |_| {}).await
    } else if let Some(file) = args.file {
        downloader.set_temp_directory(format!(
            "{}-{}.temp",
            PathBuf::from(&file)
                .file_name()
                .unwrap_or(OsStr::new("unknown"))
                .to_str()
                .unwrap_or("unknown"),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::new(0, 0))
                .as_millis()
        ));
        downloader.process_file(file, |_| {}).await
    } else {
        // Log an error if neither an ID nor a file is specified and exit
        error!("You must specify a url or file to download");
        exit(1);
    } {
        Ok(manifest) => manifest,
        Err(err) => {
            // Log an error and exit if processing fails
            error!("Failed to process pack: {}", err);
            exit(1);
        }
    };

    // Calculate and log the duration of the process
    let end_time = std::time::SystemTime::now();
    let duration = end_time.duration_since(start_time).unwrap_or_else(|err| {
        error!("Unable to get current time: {}", err);
        std::time::Duration::new(0, 0)
    });
    info!("Finished in {} seconds", duration.as_secs());
}
