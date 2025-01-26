use std::{fs::File, path::PathBuf, str::FromStr};

#[allow(unused)]
use numng;
use numng::{ConnectionPolicy, NumngError};

const BASEDIR_PATH: &str = "/home/shae/git/nulibs/numng_rs/tmpdb";
const NUMNG_JSON_PATH: &str = "/home/shae/git/nulibs/numng_rs/test/numng.json";

fn main() -> Result<(), NumngError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::debug!("Debug log visible");
    log::trace!("Trace log visible");

    let json: serde_json::Value =
        serde_json::from_reader(File::open(NUMNG_JSON_PATH).expect("Failed to read numng.json"))
            .expect("Failed to parse json");

    let (package_collection, package_id) = match numng::parse_numng_json(
        &json,
        &PathBuf::from_str(BASEDIR_PATH).expect("Failed to generate basedir pathbuf"),
        &ConnectionPolicy::Download,
        true, // use registry
        Some(false),
    ) {
        Ok(v) => v,
        Err(err) => {
            println!("{}", err);
            return Ok(());
        }
    };
    dbg!(&package_collection);
    dbg!(&package_id);
    dbg!(package_collection.get_package(package_id));

    // TODO
    Ok(())
}
