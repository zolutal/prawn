use std::fmt::Display;
use colored::Colorize;
use crate::context;


#[derive(Clone, Copy, Default)]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warning,
    Error,
    Critical
}


fn level_to_val(level: LogLevel) -> i32 {
    match level {
        LogLevel::Debug    => 0,
        LogLevel::Info     => 1,
        LogLevel::Warning  => 2,
        LogLevel::Error    => 3,
        LogLevel::Critical => 4,
    }
}

fn get_log_level() -> LogLevel {
    context::access(|ctx| {
        ctx.log_level
    })
}

fn should_log(log_level: LogLevel) -> bool {
    let curr_level = level_to_val(get_log_level());
    let req_level = level_to_val(log_level);
    if req_level >= curr_level {
        return true;
    }
    false
}

pub fn debug<T: Display>(msg: T) {
    if should_log(LogLevel::Debug) {
        println!("[{}] {}", "DEBUG".red(), msg);
    }
}

pub fn info<T: Display>(msg: T) {
    if should_log(LogLevel::Info) {
        println!("[{}] {}", "+".blue(), msg);
    }
}

pub fn warn<T: Display>(msg: T) {
    if should_log(LogLevel::Warning) {
        println!("[{}] {}", "!".yellow(), msg);
    }
}

pub fn error<T: Display>(msg: T) {
    if should_log(LogLevel::Error) {
        println!("[{}] {}", "ERROR".white().on_red(), msg);
    }
}

pub fn critical<T: Display>(msg: T) {
    if should_log(LogLevel::Critical) {
        println!("[{}] {}", "CRITICAL".white().on_red(), msg);
    }
}

