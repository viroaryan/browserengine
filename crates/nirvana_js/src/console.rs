use std::sync::{Arc, Mutex};
use boa_engine::{Context, JsResult};
use boa_gc::{Finalize, Trace};
use boa_wintertc::console::{Console, ConsoleState, Logger};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Log,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
}

pub type LogStore = Arc<Mutex<Vec<LogEntry>>>;

#[derive(Debug, Clone, Trace, Finalize)]
pub struct MemoryLogger {
    #[unsafe_ignore_trace]
    store: LogStore,
}

impl MemoryLogger {
    pub fn new(store: LogStore) -> Self {
        Self { store }
    }
}

impl Logger for MemoryLogger {
    fn log(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        if let Ok(mut lock) = self.store.lock() {
            lock.push(LogEntry {
                level: LogLevel::Log,
                message: msg,
            });
        }
        Ok(())
    }

    fn info(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        if let Ok(mut lock) = self.store.lock() {
            lock.push(LogEntry {
                level: LogLevel::Info,
                message: msg,
            });
        }
        Ok(())
    }

    fn warn(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        if let Ok(mut lock) = self.store.lock() {
            lock.push(LogEntry {
                level: LogLevel::Warn,
                message: msg,
            });
        }
        Ok(())
    }

    fn error(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        if let Ok(mut lock) = self.store.lock() {
            lock.push(LogEntry {
                level: LogLevel::Error,
                message: msg,
            });
        }
        Ok(())
    }
}

pub fn init_console(context: &mut Context, store: LogStore) -> JsResult<()> {
    let logger = MemoryLogger::new(store);
    Console::register_with_logger(logger, context)
}
