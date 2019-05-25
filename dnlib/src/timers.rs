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

        inner_log(file, module_path, line, format_args!("Starting: {}", name));

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

    /// Outputs a log message showing the current elapsed time, but does not stop the timer.
    /// This method can be called multiple times until the timer is dropped.
    /// The message includes only the elapsed time. To include more informmation, use
    /// the 'progress!' macro or the progress() method.
    pub fn log(&self) {
        inner_log(self.file,
            self.module_path,
            self.line,
            format_args!("Executing: {}, Elapsed={:?}", self.name, self.elapsed()));
    }

    pub fn progress(&self, args: fmt::Arguments) {
        inner_log(self.file,
            self.module_path,
            self.line,
            format_args!("Executing: {}, Elapsed={:?} {}", self.name, self.elapsed(), args));
    }

    /// Outputs a 'Completed' log message and suppresses the normal message that is
    /// output when the timer is dropped. This method is normally called using the
    /// 'finish!' macro. Calling finish again will have no effect.
    pub fn finish(&self, args: fmt::Arguments) {
        if !self.finished.load(Ordering::SeqCst) {
            self.finished.store(true, Ordering::SeqCst);

            inner_log(self.file,
                self.module_path,
                self.line,
                format_args!("Completed: {}, Elapsed={:?} {}", self.name, self.elapsed(), args));
        }
    }
}

impl<'a> Drop for LoggingTimer<'a> {
    fn drop(&mut self) {
        self.finish(format_args!(""));
    }
}

#[inline]
fn inner_log(
    file: &str,
    module_path: &str,
    line: u32,
    args: fmt::Arguments)
{
    log::logger().log(&
        RecordBuilder::new()
            .level(Level::Debug)
            .target("Timer")
            .file(Some(file))
            .module_path(Some(module_path))
            .line(Some(line))
            .args(args)
            .build()
    );
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

#[macro_export]
macro_rules! finish {
    ($timer:expr) => ({
        $timer.finish(format_args!(""))
    });

    ($timer:expr, $format:tt) => ({
        $timer.finish(format_args!($format))
    });

    ($timer:expr, $format:tt, $($arg:expr),*) => ({
        $timer.finish(format_args!($format, $($arg), *))
    })
}

#[macro_export]
macro_rules! progress {
    ($timer:expr) => ({
        $timer.progress(format_args!(""))
    });

    ($timer:expr, $format:tt) => ({
        $timer.progress(format_args!($format))
    });

    ($timer:expr, $format:tt, $($arg:expr),*) => ({
        $timer.progress(format_args!($format, $($arg), *))
    })
}
