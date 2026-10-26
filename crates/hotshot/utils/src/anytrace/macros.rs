/// Print the file and line number of the location this macro is invoked
///
/// Note: temporarily prints only a null string to reduce verbosity of logging
#[macro_export]
macro_rules! line_info {
    () => {
        format!("")
    };
}
pub use line_info;

/// Create an error at the trace level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
#[macro_export]
macro_rules! trace {
  ($message:literal) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Trace,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Trace,
        message: format!("{}: {}", $crate::anytrace::line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Trace,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use trace;

/// Create an error at the debug level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
#[macro_export]
macro_rules! debug {
  ($message:literal) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Debug,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Debug,
        message: format!("{}: {}", $crate::anytrace::line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Debug,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use debug;

/// Create an error at the info level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
#[macro_export]
macro_rules! info {
  ($message:literal) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Info,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Info,
        message: format!("{}: {}", $crate::anytrace::line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Info,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use info;

/// Create an error at the warn level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
#[macro_export]
macro_rules! warn {
  ($message:literal) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Warn,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Warn,
        message: format!("{}: {}", $crate::anytrace::line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Warn,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use crate::warn;

/// Create an error at the error level.
///
/// The argument can be either:
///   - an expression implementing `Display`
///   - a string literal
///   - a format string, similar to the `format!()` macro
#[macro_export]
macro_rules! error {
  ($message:literal) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Error,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($message))
      }
  };
  ($error:expr) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Error,
        message: format!("{}: {}", $crate::anytrace::line_info!(), $error)
      }
  };
  ($fmt:expr, $($arg:tt)*) => {
      $crate::anytrace::Error {
        level: $crate::anytrace::Level::Error,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($fmt, $($arg)*))
      }
  };
}
pub use error;

/// Log an `Error` at the corresponding level.
#[macro_export]
macro_rules! log {
    ($result:expr) => {
        if let Err(ref error) = $result {
            let mut error_level = error.level;
            if error_level == $crate::anytrace::Level::Unspecified {
                error_level = $crate::anytrace::DEFAULT_LOG_LEVEL;
            }

            match error_level {
                $crate::anytrace::Level::Trace => {
                    tracing::trace!("{}", error.message);
                },
                $crate::anytrace::Level::Debug => {
                    tracing::debug!("{}", error.message);
                },
                $crate::anytrace::Level::Info => {
                    tracing::info!("{}", error.message);
                },
                $crate::anytrace::Level::Warn => {
                    tracing::warn!("{}", error.message);
                },
                $crate::anytrace::Level::Error => {
                    tracing::error!("{}", error.message);
                },
                // impossible
                $crate::anytrace::Level::Unspecified => {},
            }
        }
    };
}
pub use log;

/// Check that the given condition holds, otherwise return an error.
///
/// The argument can be either:
///   - a condition, in which case a generic error is logged at the `Unspecified` level.
///   - a condition and a string literal, in which case the provided literal is logged at the `Unspecified` level.
///   - a condition and a format expression, in which case the message is formatted and logged at the `Unspecified` level.
///   - a condition and an `Error`, in which case the given error is logged unchanged.
#[macro_export]
macro_rules! ensure {
  ($condition:expr) => {
      if !$condition {
        let result = Err($crate::anytrace::Error {
          level: $crate::anytrace::Level::Unspecified,
          message: format!("{}: condition '{}' failed.", $crate::anytrace::line_info!(), stringify!($condition))
        });

        $crate::anytrace::log!(result);

        return result;
     }
  };
  ($condition:expr, $message:literal) => {
      if !$condition {
        let result = Err($crate::anytrace::Error {
          level: $crate::anytrace::Level::Unspecified,
          message: format!("{}: {}", $crate::anytrace::line_info!(), format!($message))
        });

        $crate::anytrace::log!(result);

        return result;
      }
  };
  ($condition:expr, $fmt:expr, $($arg:tt)*) => {
      if !$condition {
        let result = Err($crate::anytrace::Error {
          level: $crate::anytrace::Level::Unspecified,
          message: format!("{}: {}", $crate::anytrace::line_info!(), format!($fmt, $($arg)*))
        });

        $crate::anytrace::log!(result);

        return result;
      }
  };
  ($condition:expr, $error:expr) => {
      if !$condition {
        let result = Err($error);

        $crate::anytrace::log!(result);

        return result;
      }
  };
}
pub use ensure;

/// Return an error.
///
/// The argument can be either:
///   - nothing, in which case a generic message is logged at the `Unspecified` level.
///   - a string literal, in which case the provided literal is logged at the `Unspecified` level.
///   - a format expression, in which case the message is formatted and logged at the `Unspecified` level.
///   - an `Error`, in which case the given error is logged unchanged.
#[macro_export]
macro_rules! bail {
  () => {
      let result = Err($crate::anytrace::Error {
        level: $crate::anytrace::Level::Unspecified,
        message: format!("{}: bailed.", $crate::anytrace::line_info!()),
      });

      $crate::anytrace::log!(result);

      return result;
  };
  ($message:literal) => {
      let result = Err($crate::anytrace::Error {
        level: $crate::anytrace::Level::Unspecified,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($message))
      });

      $crate::anytrace::log!(result);

      return result;
  };
  ($fmt:expr, $($arg:tt)*) => {
      let result = Err($crate::anytrace::Error {
        level: $crate::anytrace::Level::Unspecified,
        message: format!("{}: {}", $crate::anytrace::line_info!(), format!($fmt, $($arg)*))
      });

      $crate::anytrace::log!(result);

      return result;
  };
  ($error:expr) => {
      let result = Err($error);

      $crate::anytrace::log!(result);

      return result;
  };
}
pub use bail;
