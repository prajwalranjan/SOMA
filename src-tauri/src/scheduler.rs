use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SCHEDULER_INTERVAL_SECS: u64 = 60 * 60 * 6;

pub fn start_scheduler(conn: Arc<Mutex<Connection>>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(SCHEDULER_INTERVAL_SECS));

        let conn_guard = conn.lock().unwrap();
        match crate::clustering::run_clustering(&conn_guard) {
            Ok(clusters) => println!("Scheduler: found {} clusters", clusters.len()),
            Err(e) => eprintln!("Scheduler error: {}", e),
        }
    });
}
