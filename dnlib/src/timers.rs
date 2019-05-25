use std::time::Instant;
use std::fmt;
use log::{RecordBuilder, Level};
use std::sync::atomic::{AtomicBool, Ordering};

/// When this struct is dropped, it logs a message stating its name and how long, in seconds,
/// execution time was. Can be used to time functions or other critical areas.
pub struct LoggingTimer<'a> {
    start_time: Instant,
    file: &'static str,
    module_path: &'static str,
    line: u32,
    name: &'a str,
    finished: AtomicBool,
}

const TARGET: &'static str = "Timer";
const LOG_LEVEL: Level = Level::Debug;

impl<'a> LoggingTimer<'a> {
    pub fn new(name: &'a str,
        file: &'static str,
        module_path: &'static str,
        line: u32
        ) -> Self
    {
        LoggingTimer {
            start_time: Instant::now(),
            file: file,
            module_path: module_path,
            line: line,
            name: name,
            finished: AtomicBool::new(false),
        }
    }

    // Construct a new ExecutionTimer and prints a message saying execution is starting.
    pub fn with_start_message(name: &'a str,
        file: &'static str,
        module_path: &'static str,
        line: u32
        ) -> Self
    {
        // Determine this before calling log(), since debug!() will take time
        // itself, i.e. it is overhead that can confuse timings.
        let start_time = Instant::now();

        log::logger().log(&
            RecordBuilder::new()
                .level(LOG_LEVEL)
                .target(TARGET)
                .file(Some(file))
                .module_path(Some(module_path))
                .line(Some(line))
                .args(format_args!("Starting: {}", name))
                .build()
        );

        LoggingTimer {
            start_time: start_time,
            file: file,
            module_path: module_path,
            line: line,
            name: name,
            finished: AtomicBool::new(false),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Outputs a log message showing the current elapsed time,
    /// but does not stop the timer. This method can be called multiple times
    /// until the timer is dropped.
    pub fn log(&self) {
        let elapsed = self.start_time.elapsed();

        log::logger().log(&
            RecordBuilder::new()
                .level(LOG_LEVEL)
                .target(TARGET)
                .file(Some(self.file))
                .module_path(Some(self.module_path))
                .line(Some(self.line))
                .args(format_args!("Executing: {}, Elapsed={:?}", self.name, elapsed))
                .build()
        );
    }

    pub fn finish(&self, args: fmt::Arguments) {
        self.finished.store(true, Ordering::SeqCst);

        log::logger().log(&
            RecordBuilder::new()
                .level(LOG_LEVEL)
                .target(TARGET)
                .file(Some(self.file))
                .module_path(Some(self.module_path))
                .line(Some(self.line))
                .args(args)
                .build()
        );

        // log::logger().log(&
        //     RecordBuilder::new()
        //         .level(LOG_LEVEL)
        //         .target(TARGET)
        //         .file(Some(self.file))
        //         .module_path(Some(self.module_path))
        //         .line(Some(self.line))
        //         .args(format_args!("Completed: {}, Elapsed={:?}", self.name, elapsed))
        //         .build()
        // );
    }
}

impl<'a> Drop for LoggingTimer<'a> {
    fn drop(&mut self) {
        if !self.finished.load(Ordering::SeqCst) {
            self.finish(format_args!("Completed: {}, Elapsed={:?}", self.name, self.elapsed()));
        }

        //

        // log::logger().log(&
        //     RecordBuilder::new()
        //         .level(LOG_LEVEL)
        //         .target(TARGET)
        //         .file(Some(self.file))
        //         .module_path(Some(self.module_path))
        //         .line(Some(self.line))
        //         .args(format_args!("Completed: {}, Elapsed={:?}", self.name, elapsed))
        //         .build()
        // );
    }
}

/// Creates a timer that does not log a starting message, only a completed one.
#[macro_export]
macro_rules! timer {
    ($str:expr) => {
        {
            crate::LoggingTimer::new(
                $str,
                file!(),
                module_path!(),
                line!()
                )
        }
    }
}

/// Creates a timer that logs a starting mesage and a completed message.
#[macro_export]
macro_rules! stimer {
    ($str:expr) => {
        {
            crate::LoggingTimer::with_start_message(
                $str,
                file!(),
                module_path!(),
                line!()
                )
        }
    }
}
