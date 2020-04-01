use crate::error::{ErrorKind, Result};
use fern;
use log::Level;
use std::io;

pub fn setup_logging(_for_crate: String, log_level: Option<Level>) -> Result<()> {
    let log_level_filter = log_level.unwrap_or(Level::Trace).to_level_filter();

    let logging = fern::Dispatch::new()
        .level(log_level_filter)
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                chrono::Local::now().format("%H:%M"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(io::stdout())
        .apply();

    match logging {
        Err(_) => Err(ErrorKind::LoggingSetupFailed)?,
        Ok(_) => Ok(()),
    }
}
