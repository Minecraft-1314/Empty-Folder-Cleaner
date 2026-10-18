pub const UNKNOWN: &str = "operation_failed";
pub const PERMISSION_DENIED: &str = "permission_denied";
pub const NOT_FOUND: &str = "not_found";
pub const PATH_TOO_LONG: &str = "path_too_long";
pub const IO_ERROR: &str = "io_error";
pub const CANCELLED: &str = "cancelled";
pub const TIMEOUT: &str = "timeout";
pub const INVALID_INPUT: &str = "invalid_input";
pub const BUSY: &str = "operation_busy";

pub fn from_io(error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => PERMISSION_DENIED.to_string(),
        std::io::ErrorKind::NotFound => NOT_FOUND.to_string(),
        std::io::ErrorKind::TimedOut => TIMEOUT.to_string(),
        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => {
            INVALID_INPUT.to_string()
        }
        _ => match error.raw_os_error() {
            Some(206) | Some(3) => PATH_TOO_LONG.to_string(),
            _ => IO_ERROR.to_string(),
        },
    }
}
