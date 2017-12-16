use chrono::prelude::*;

use simplelog::*;

use std::path::PathBuf;
use std::fs::{File, create_dir_all};

pub fn prepare_test_directory(test_directory_name: &str) -> PathBuf {
    let test_output_parent_directory = "test_output";

    let mut path_buf = PathBuf::from(test_output_parent_directory);
    path_buf.push(format!("{}-{}", filename_timestamp(), test_directory_name));

    create_dir_all(&path_buf).expect("Test output directory could not be created");

    let log_file = create_log_file(&mut path_buf);

    CombinedLogger::init(
        vec![
            TermLogger::new(LogLevelFilter::Info, Config::default()).unwrap(),
            WriteLogger::new(LogLevelFilter::Trace, Config::default(), log_file),
        ]
    ).unwrap();

    info!("Created test directory {:?} and initialized logging", path_buf);

    path_buf
}

fn create_log_file(parent_directory: &mut PathBuf) -> File {
    parent_directory.push(format!("log-{}", filename_timestamp()));
    parent_directory.set_extension("log");

    let log_file = File::create(&parent_directory).expect("Log file could not be created");
    // Restore state before pushing and setting extension
    parent_directory.pop();

    log_file
}

/// Returns the current time formatted like "2014-11-28T120009+0000", i.e.
/// an ISO 8601 timestamp with the colons removed, since colons are traditionally
/// used as directory separators on mac and linux
fn filename_timestamp() -> String {
    Utc::now()
        .to_rfc3339()
        .replace(":", "")
}

