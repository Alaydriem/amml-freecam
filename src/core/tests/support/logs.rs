use std::sync::Mutex;

use curia::{Dispatcher, Level, LogEvent, Logger, Sink};

// a Curia sink that keeps every event, so a test can count the warnings it caused
pub struct Logs;

static RECORDS: Mutex<Vec<(Level, String)>> = Mutex::new(Vec::new());

impl Logs {
    // installs the sink once per test process; later calls change nothing
    pub fn install() {
        let _ = Logger::install(Box::new(Dispatcher::new(vec![Logs])));
    }

    // events at `level` whose message contains `text`
    pub fn count(level: Level, text: &str) -> usize {
        let records = RECORDS.lock().unwrap();
        records.iter().filter(|(at, message)| *at == level && message.contains(text)).count()
    }
}

impl Sink for Logs {
    fn level(&self) -> Level {
        Level::Trace
    }

    fn emit(&self, event: &LogEvent) {
        RECORDS.lock().unwrap().push((event.level, event.message.clone()));
    }
}
