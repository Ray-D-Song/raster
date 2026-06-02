use std::{
    fs,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use super::super::{LogLevel, Logger, LoggerConfig};

#[test]
fn warn_level_does_not_print_info() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let logger = Logger::new_for_test(
        LoggerConfig {
            level: LogLevel::Warn,
            file_path: None,
        },
        output.clone(),
    )
    .expect("test logger");

    logger.info("hidden info");
    logger.warn("visible warn");

    let output = String::from_utf8(output.lock().expect("output lock").clone()).expect("utf8");
    assert!(!output.contains("hidden info"));
    assert!(output.contains("[warn] visible warn"));
}

#[test]
fn logger_writes_to_file_when_path_is_configured() {
    let file_path = std::env::temp_dir().join(format!(
        "raster-logger-test-{}.log",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    let output = Arc::new(Mutex::new(Vec::new()));
    let logger = Logger::new_for_test(
        LoggerConfig {
            level: LogLevel::Info,
            file_path: Some(file_path.clone()),
        },
        output,
    )
    .expect("test logger");

    logger.info("file output");

    let file = fs::read_to_string(&file_path).expect("read log file");
    assert!(file.contains("[info] file output"));

    let _ = fs::remove_file(file_path);
}
