use chrono::Local;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use crate::config::LogConfig;
use crate::Result;

pub struct Logger {
    config: LogConfig,
    event_log: Mutex<File>,
    message_log: Mutex<File>,
}

impl Logger {
    pub fn new(config: &LogConfig) -> Self {
        let event_log = Self::open_log_file(
            &config.log_directory,
            "event.log"
        ).expect("Failed to open event log");
        
        let message_log = Self::open_log_file(
            &config.log_directory,
            "message.log"
        ).expect("Failed to open message log");

        Logger {
            config: config.clone(),
            event_log: Mutex::new(event_log),
            message_log: Mutex::new(message_log),
        }
    }

    pub fn log_event(&self, level: &str, message: &str) -> Result<()> {
        if !self.config.log_events {
            return Ok(());
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] {} - {}\n", timestamp, level, message);
        
        let mut log = self.event_log.lock().unwrap();
        log.write_all(log_line.as_bytes())?;
        log.flush()?;
        
        Ok(())
    }

    pub fn log_message(&self, direction: &str, message: &str) -> Result<()> {
        if !self.config.log_messages {
            return Ok(());
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] {} - {}\n", timestamp, direction, message);
        
        let mut log = self.message_log.lock().unwrap();
        log.write_all(log_line.as_bytes())?;
        log.flush()?;
        
        Ok(())
    }

    fn open_log_file(dir: &Path, filename: &str) -> std::io::Result<File> {
        let path = dir.join(filename);
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    }
}
