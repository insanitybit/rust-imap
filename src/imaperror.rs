#![deny(warnings)]
extern crate openssl;

use std::error;
use std::fmt;
use std::io::Error as ioError;
use openssl::ssl::error::*;

#[derive(Debug)]
pub enum IMAPError {
    IOError(ioError),
    SslError(SslError),
    LoginError(String),
    SelectError(String),
    ConnectError(String),
    No(String),
    Bad(String),
    Invalid(String)
}

impl fmt::Display for IMAPError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IMAPError::IOError(ref err) => write!(f, "IO error: {}", err),
            IMAPError::SslError(ref err) => write!(f, "Ssl error: {}", err),
            IMAPError::LoginError(ref err) => write!(f, "Login error: {}", err),
            IMAPError::SelectError(ref err) => write!(f, "Select error: {}", err),
            IMAPError::ConnectError(ref err) => write!(f, "Connect error: {}", err),
            IMAPError::No(ref err) => write!(f, "Connect error: {}", err),
            IMAPError::Bad(ref err) => write!(f, "Connect error: {}", err),
            IMAPError::Invalid(ref err) => write!(f, "Connect error: {}", err),
        }
    }
}

impl error::Error for IMAPError {

    fn description(&self) -> &str {
        match *self {
            IMAPError::IOError(ref err) => err.description(),
            IMAPError::SslError(ref err) => err.description(),
            IMAPError::LoginError(ref err) => err,
            IMAPError::SelectError(ref err) => err,
            IMAPError::ConnectError(ref err) => err,
            IMAPError::No(ref err) => err,
            IMAPError::Bad(ref err) => err,
            IMAPError::Invalid(ref err) => err,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            IMAPError::IOError(ref err) => Some(err),
            IMAPError::SslError(ref err) => Some(err),
            IMAPError::LoginError(_) => None,
            IMAPError::SelectError(_) => None,
            IMAPError::ConnectError(_) => None,
            IMAPError::No(_) => None,
            IMAPError::Bad(_) => None,
            IMAPError::Invalid(_) => None,
        }
    }
}

impl From<ioError> for IMAPError {
    fn from(err: ioError) -> IMAPError {
        IMAPError::IOError(err)
    }
}

impl From<SslError> for IMAPError {
    fn from(err: SslError) -> IMAPError {
        IMAPError::SslError(err)
    }
}
