use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SCHEDULER_INTERVAL_SECS: u64 = 60 * 60 * 6;

pub fn start_scheduler(_conn: Arc<Mutex<Connection>>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(SCHEDULER_INTERVAL_SECS));
        println!("Scheduler: tick — insight generation deferred to user request");
    });
}
