#[macro_export]
/// Print the file and line number of the location this macro is invoked
///
/// Note: temporarily prints only a null string to reduce verbosity of logging
macro_rules! line_info {
    () => {
        format!("")
    };
}
pub use line_info;

#[macro_export]
/// Create an error at the trace level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
macro_rules! trace {
  ($message:literal) => {
      Error {
        level: Level::Trace,
        message: format!("{}: {}", line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      Error {
        level: Level::Trace,
        message: format!("{}: {}", line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      Error {
        level: Level::Trace,
        message: format!("{}: {}", line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use trace;

#[macro_export]
/// Create an error at the debug level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
macro_rules! debug {
  ($message:literal) => {
      Error {
        level: Level::Debug,
        message: format!("{}: {}", line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      Error {
        level: Level::Debug,
        message: format!("{}: {}", line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      Error {
        level: Level::Debug,
        message: format!("{}: {}", line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use debug;

#[macro_export]
/// Create an error at the info level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
macro_rules! info {
  ($message:literal) => {
      Error {
        level: Level::Info,
        message: format!("{}: {}", line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      Error {
        level: Level::Info,
        message: format!("{}: {}", line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      Error {
        level: Level::Info,
        message: format!("{}: {}", line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use info;

#[macro_export]
/// Create an error at the warn level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
macro_rules! warn {
  ($message:literal) => {
      Error {
        level: Level::Warn,
        message: format!("{}: {}", line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      Error {
        level: Level::Warn,
        message: format!("{}: {}", line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      Error {
        level: Level::Warn,
        message: format!("{}: {}", line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use crate::warn;

#[macro_export]
/// Create an error at the error level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
macro_rules! error {
  ($message:literal) => {
      Error {
        level: Level::Error,
        message: format!("{}: {}", line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      Error {
        level: Level::Error,
        message: format!("{}: {}", line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      Error {
        level: Level::Error,
        message: format!("{}: {}", line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use error;

#[macro_export]
/// Log a `anytrace::Error` at the corresponding level.
macro_rules! log {
    ($result:expr) => {
        if let Err(ref error) = $result {
            let mut error_level = error.level;
            if error_level == Level::Unspecified {
                error_level = DEFAULT_LOG_LEVEL;
            }

            match error_level {
                Level::Trace => {
                    tracing::trace!("{}", error.message);
                },
                Level::Debug => {
                    tracing::debug!("{}", error.message);
                },
                Level::Info => {
                    tracing::info!("{}", error.message);
                },
                Level::Warn => {
                    tracing::warn!("{}", error.message);
                },
                Level::Error => {
                    tracing::error!("{}", error.message);
                },
                // impossible
                Level::Unspecified => {},
            }
        }
    };
}
pub use log;

#[macro_export]
/// Check that the given condition holds, otherwise return an error.
///
/// The argument can be either:
///   - a condition, in which case a generic error is logged at the `Unspecified` level.
///   - a condition and a string literal, in which case the provided literal is logged at the `Unspecified` level.
///   - a condition and a format expression, in which case the message is formatted and logged at the `Unspecified` level.
///   - a condition and an `Error`, in which case the given error is logged unchanged.
macro_rules! ensure {
  ($condition:expr) => {
      if !$condition {
        let result = Err(Error {
          level: Level::Unspecified,
          message: format!("{}: condition '{}' failed.", line_info!(), stringify!($condition))
        });

        log!(result);

        return result;
     }
  };
  ($condition:expr, $message:literal) => {
      if !$condition {
        let result = Err(Error {
          level: Level::Unspecified,
          message: format!("{}: {}", line_info!(), format!($message))
        });

        log!(result);

        return result;
      }
  };
  ($condition:expr, $fmt:expr, $($arg:tt)*) => {
      if !$condition {
        let result = Err(Error {
          level: Level::Unspecified,
          message: format!("{}: {}", line_info!(), format!($fmt, $($arg)*))
        });

        log!(result);

        return result;
      }
  };
  ($condition:expr, $error:expr) => {
      if !$condition {
        let result = Err($error);

        log!(result);

        return result;
      }
  };
}
pub use ensure;

#[macro_export]
/// Return an error.
///
/// The argument can be either:
///   - nothing, in which case a generic message is logged at the `Unspecified` level.
///   - a string literal, in which case the provided literal is logged at the `Unspecified` level.
///   - a format expression, in which case the message is formatted and logged at the `Unspecified` level.
///   - an `Error`, in which case the given error is logged unchanged.
macro_rules! bail {
  () => {
      let result = Err(Error {
        level: Level::Unspecified,
        message: format!("{}: bailed.", line_info!()),
      });

      log!(result);

      return result;
  };
  ($message:literal) => {
      let result = Err(Error {
        level: Level::Unspecified,
        message: format!("{}: {}", line_info!(), format!($message))
      });

      log!(result);

      return result;
  };
  ($fmt:expr, $($arg:tt)*) => {
      let result = Err(Error {
        level: Level::Unspecified,
        message: format!("{}: {}", line_info!(), format!($fmt, $($arg)*))
      });

      log!(result);

      return result;
  };
  ($error:expr) => {
      let result = Err($error);

      log!(result);

      return result;
  };
}
pub use bail;
