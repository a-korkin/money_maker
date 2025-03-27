use chrono::offset::Local;
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

#[allow(dead_code)]
struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let time = Local::now().format("%H:%M:%S");
            let color = match record.level() {
                Level::Warn => "33m",
                Level::Error => "31m",
                Level::Info => "32m",
                _ => "37m",
            };
            println!(
                "\x1b[{color}[{}] {}: {}\x1b[0m",
                time,
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

#[allow(dead_code)]
static LOGGER: SimpleLogger = SimpleLogger;

#[allow(dead_code)]
pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info))
}
