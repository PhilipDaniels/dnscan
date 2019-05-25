use std::time::Instant;
use log::{RecordBuilder, Level};


// When this struct is dropped, it logs a message stating its name and how long, in seconds,
// execution time was. Can be used to time functions or other critical areas.
pub struct ExecutionTimer<'a> {
	start_time: Instant,
    file: &'static str,
    module_path: &'static str,
    line: u32,
	name: &'a str
}

impl<'a> ExecutionTimer<'a> {
	pub fn new(name: &'a str,
        file: &'static str,
        module_path: &'static str,
        line: u32
        ) -> Self
    {
		ExecutionTimer {
			start_time: Instant::now(),
            file: file,
            module_path: module_path,
            line: line,
			name: name
		}
	}

	// // Construct a new ExecutionTimer and prints a message saying execution is starting.
	// pub fn with_start_message(name: String, file: &'static str) -> Self {
    //     // Determine this before calling debug!(), since debug!() will take time
    //     // itself, i.e. it is overhead that can confuse timings.
    //     let start_time = Instant::now();
	// 	debug!("Starting: {}", name);
	// 	ExecutionTimer2 { start_time, file, name }
	// }
}

impl<'a> Drop for ExecutionTimer<'a> {
	fn drop(&mut self) {
		let elapsed = self.start_time.elapsed();
        let mut builder = RecordBuilder::new();
        let logger = log::logger();

        logger.log(&
            builder
                .level(Level::Debug)
                .target("ExecutionTimer")
                .file(Some(self.file))
                .module_path(Some(self.module_path))
                .line(Some(self.line))
                .args(format_args!("Completed: {}, Elapsed={:?}", self.name, elapsed))
                .build()
        );
	}
}


/// Creates a timer that logs a starting and completed message.
#[macro_export]
macro_rules! timer {
    ($str:expr) => { crate::ExecutionTimer::with_start_message($str) }
}

/// Creates a quiet timer that does not log a starting message, only a completed one.
#[macro_export]
macro_rules! qtimer {
    ($str:expr) => {
        {
            crate::ExecutionTimer::new(
                $str,
                file!(),
                module_path!(),
                line!()
                )
        }
    }
}
