use std::time::Instant;
use std::fmt;
use log::{RecordBuilder, Level};
use std::sync::atomic::{AtomicBool, Ordering};

/// When this struct is dropped, it logs a message stating its name and how long
/// the execution time was. Can be used to time functions or other critical areas.
pub struct LoggingTimer<'a> {
    /// Set by the file!() macro to the name of the file where the timer is instantiated.
    file: &'static str,
    /// Set by the module_path!() macro to the module where the timer is instantiated.
    module_path: &'static str,
    /// Set by the line!() macro to the line number where the timer is instantiated.
    line: u32,
    /// A flag used to suppress printing of the 'Completed' message in the drop() function
    /// It is set by the finish method.
    finished: AtomicBool,
    /// The instant, in UTC, that the timer was instantiated.
    start_time: Instant,
    /// The name of the timer. Used in messages to identify it.
    name: &'a str,
}

impl<'a> LoggingTimer<'a> {
    /// Constructs a new `LoggingTimer` that prints only a 'Completed' message.
    /// This method is not usually called directly, use the `timer!` macro instead.
    pub fn new(
        file: &'static str,
        module_path: &'static str,
        line: u32,
        name: &'a str,
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

    /// Constructs a new `LoggingTimer` that prints a 'Starting' and a 'Completed' message.
    /// This method is not usually called directly, use the `stimer!` macro instead.
    pub fn with_start_message(
        file: &'static str,
        module_path: &'static str,
        line: u32,
        name: &'a str,
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

    /// Returns how long the timer has been running for.
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

    /// Outputs a log message showing the current elapsed time, but does not stop the timer.
    /// This method can be called multiple times until the timer is dropped.
    /// The message can include further information via a `format_args!` approach.
    /// This method is usually not called directly, it is easier to call via the `progress!`
    /// macro.
    pub fn progress(&self, args: fmt::Arguments) {
        inner_log(self.file,
            self.module_path,
            self.line,
            format_args!("Executing: {}, Elapsed={:?} {}", self.name, self.elapsed(), args));
    }

    /// Outputs a 'Completed' log message and suppresses the normal message that is
    /// output when the timer is dropped. The message can include further `format_args!`
    /// information. This method is normally called using the `finish!` macro. Calling
    /// finish() again will have no effect.
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
    /// Drops the timer, outputting a 'Completed' message if `finish` has not yet
    /// been called.
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
    ($name:expr) => {
        {
            crate::LoggingTimer::new(
                file!(),
                module_path!(),
                line!(),
                $name,
                )
        }
    }
}

/// Creates a timer that logs a starting mesage and a completed message.
#[macro_export]
macro_rules! stimer {
    ($name:expr) => {
        {
            crate::LoggingTimer::with_start_message(
                file!(),
                module_path!(),
                line!(),
                $name,
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
