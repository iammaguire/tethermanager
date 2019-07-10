use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use simple_error::SimpleError;

const DEFAULT_TIMEOUT: u64 = 1;

fn connect(addr: &SocketAddr, timeout: Option<Duration>) -> Result<bool, SimpleError> {
    let duration = match timeout {
        Some(tout) => tout,
        _ => Duration::new(DEFAULT_TIMEOUT, 0),
    };

    match TcpStream::connect_timeout(addr, duration) {
        Ok(_) => Ok(true),
        Err(e) => Err(SimpleError::from(e)),
    }
}

/// It uses HTTP and DNS as fallback.
///
/// * `timeout` - Number of seconds to wait for a response (default: 3)
pub fn online(timeout: Option<Duration>) -> Result<bool, SimpleError> {
    //! ```rust
    //! use std::time::Duration;
    //!
    //! use online::*;
    //!
    //! assert_eq!(online(None), Ok(true));
    //!
    //! // with timeout
    //! let timeout = Duration::new(6, 0);
    //! assert_eq!(online(Some(timeout)), Ok(true));
    //! ```

    // Chrome captive portal detection.
    // http://clients3.google.com/generate_204
    let addr = SocketAddr::from(([216, 58, 201, 174], 80));

    match connect(&addr, timeout) {
        Ok(_) => Ok(true),
        Err(e) => match e.as_str() {
            "Network is unreachable (os error 101)" => Ok(false),
            "connection timed out" => {
                // Firefox captive portal detection.
                // http://detectportal.firefox.com/success.txt.
                let addr_fallback = SocketAddr::from(([2, 22, 126, 57], 80));

                match connect(&addr_fallback, timeout) {
                    Ok(_) => Ok(true),
                    Err(err) => match err.as_str() {
                        "connection timed out" => Ok(false),
                        _ => Err(err),
                    },
                }
            }
            _ => Err(e),
        },
    }
}
