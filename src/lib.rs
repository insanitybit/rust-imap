#![allow(unused_variables, unused_imports, dead_code)]
extern crate openssl;
extern crate rand;

pub mod imaperror;
use imaperror::IMAPError;

use std::net::TcpStream;
use openssl::ssl::{SslContext, SslStream, SslMethod, Ssl};
use std::io::prelude;
use std::io::{Write, Read};
use std::time::Duration;
use rand::Rng;

#[derive(Debug)]
pub enum IMAPConnection {
    Basic(TcpStream),
    Ssl(SslStream<TcpStream>),
    Disconnected,
}


#[derive(Debug)]
pub enum IMAPClient {
    Authenticated(MailServer),
    UnAuthenticated(MailServer),
    Selected(Mailbox),
    Logout,
}

#[derive(Debug)]
struct Tag {
    tag_prefix: String,
    tag: u32,
}

#[derive(Debug)]
pub struct MailServer {
    imap: IMAPConnection,
    tag: Tag,
}

#[derive(Debug)]
pub struct Mailbox {
    imap: IMAPConnection,
    tag: Tag,
}

pub type SequenceSet = (u32, u32);

#[derive(Debug)]
pub enum Macro {
    All,
    Fast,
    Full,
}

impl IMAPConnection {

    pub fn new() -> IMAPConnection {
        IMAPConnection::Disconnected
    }

    pub fn new_notls<IntoString: Into<String>>(host: IntoString,
                                               port: u32)
                                               -> Result<IMAPConnection, IMAPError> {
        let host = host.into();
        let server = host + ":" + &port.to_string();

        let stream = try!(TcpStream::connect(&*server));
        let _ = try!(stream.set_read_timeout(Some(Duration::from_secs(2))));
        let _ = try!(stream.set_write_timeout(Some(Duration::from_secs(2))));

        Ok(IMAPConnection::Basic(stream))
    }

    pub fn new_tls<IntoString: Into<String>>(host: IntoString,
                                             port: u32)
                                             -> Result<IMAPConnection, IMAPError> {
        let host = host.into();
        let server = host + ":" + &port.to_string();

        let stream = {
            let stream = try!(TcpStream::connect(&*server));
            let _ = try!(stream.set_read_timeout(Some(Duration::from_secs(2))));
            let _ = try!(stream.set_write_timeout(Some(Duration::from_secs(2))));

            let sslcontext = try!(SslContext::new(SslMethod::Sslv23));
            let ssl = try!(Ssl::new(&sslcontext));
            let stream = try!(SslStream::connect(ssl, stream));

            stream
        };

        Ok(IMAPConnection::Ssl(stream))
    }
}

impl Tag {
    fn new() -> Tag {
        let mut rng = rand::thread_rng();
        let rstr: String = rng.gen_ascii_chars()
                              .take(3)
                              .collect();


        let rnum: u32 = 0;

        Tag {
            tag_prefix: rstr,
            tag: rnum,
        }
    }

    /// Increments and then returns the tag.
    fn next_tag(&mut self) -> String {
        self.tag += 1;

        (&self).tag_prefix.to_owned() + &format!("{:05}", self.tag)
    }
}

impl IMAPClient {
    pub fn connect(imap: IMAPConnection) -> Result<IMAPClient, IMAPError> {
        let mut imap = imap;

        let stream = match &mut imap {
            &mut IMAPConnection::Basic(ref mut stream) => try!(IMAPClient::read_greeting(stream)),
            &mut IMAPConnection::Ssl(ref mut stream) => try!(IMAPClient::read_greeting(stream)),
            &mut IMAPConnection::Disconnected =>
                return Err(IMAPError::ConnectError("Can not connect, IMAPConection in \
                                                    Disconnected state"
                                                       .to_owned())),
        };

        let unauthenticated = MailServer {
            imap: imap,
            tag: Tag::new(),
        };

        Ok(IMAPClient::UnAuthenticated(unauthenticated))
    }

    pub fn login<IntoString: Into<String>>(self,
                                           username: IntoString,
                                           password: IntoString)
                                           -> Result<IMAPClient, (IMAPClient, IMAPError)> {
        let cmd = format!("LOGIN {} {}", username.into(), password.into());

        match self {
            IMAPClient::UnAuthenticated(mut server) => {
                let tag = server.tag.next_tag();
                let cmd = format!("{} {}\r\n", tag, cmd);
                match server.command(&cmd) {
                    Ok(res) => {
                        match IMAPClient::check_tagged_response(res, &tag) {
                                Ok(_)   =>  Ok(IMAPClient::Authenticated(server)),
                                Err(e)  => Err((IMAPClient::UnAuthenticated(server), e))
                            }
                        },
                    Err(e) => Err((IMAPClient::UnAuthenticated(server), e)),
                }
            }
            IMAPClient::Authenticated(server) => {
                Ok(IMAPClient::Authenticated(server))
            }
            IMAPClient::Selected(mut mailbox) => {
                let tag = mailbox.tag.next_tag();
                let cmd = format!("{} {}\r\n", tag, cmd);
                match mailbox.command(&cmd) {
                    Ok(_) => Ok(IMAPClient::Selected(mailbox)),
                    Err(e) => Err((IMAPClient::Selected(mailbox), e)),
                }
            }
            IMAPClient::Logout => {
                Err((IMAPClient::Logout,
                     IMAPError::LoginError("Not valid to try to log in after Logout".to_owned())))
            }
        }

    }

    pub fn select<IntoString: Into<String>>(self,
                                            mailbox_name: IntoString)
                                            -> Result<IMAPClient, (IMAPClient, IMAPError)> {
        let cmd = format!("SELECT {}", mailbox_name.into());

        match self {
            IMAPClient::UnAuthenticated(server) => {
                Err((IMAPClient::UnAuthenticated(server),
                     IMAPError::SelectError("Must authenticate before SELECT".to_owned())))
            }
            IMAPClient::Authenticated(mut server) => {
                let tag = server.tag.next_tag();
                let cmd = format!("{} {}\r\n", tag, cmd);
                match server.command(&cmd) {
                    Ok(res) => {
                        match IMAPClient::check_response(res) {
                                Ok(_)   => {
                                    let mailbox = Mailbox {
                                        imap: server.imap,
                                        tag: server.tag,
                                    };
                                    Ok(IMAPClient::Selected(mailbox))
                                },
                                Err(e)  => Err((IMAPClient::UnAuthenticated(server), e))
                            }
                        },
                    Err(e) => Err((IMAPClient::Authenticated(server), e)),
                }
            }
            IMAPClient::Selected(mut mailbox) => {
                let tag = mailbox.tag.next_tag();
                let cmd = format!("{} {}\r\n", tag, cmd);
                match mailbox.command(&cmd) {
                    Ok(res) => {
                        match IMAPClient::check_response(res) {
                                Ok(_)   => {
                                    Ok(IMAPClient::Selected(mailbox))
                                },
                                Err(e)  => {
                                    Err((IMAPClient::Selected(mailbox), e))
                                }
                            }
                        },
                    Err(e) => Err((IMAPClient::Selected(mailbox), e)),
                }
            }
            IMAPClient::Logout => {
                Err((IMAPClient::Logout, IMAPError::SelectError("Can not select in Logout state".to_owned())))
            }
        }
    }

    pub fn logout(self) -> Result<IMAPClient, (IMAPClient, IMAPError)> {
        let cmd = format!("LOGOUT");

        match self {
            IMAPClient::UnAuthenticated(server) => {
                Ok(IMAPClient::Logout)
            }
            IMAPClient::Authenticated(mut server) => {
                let tag = server.tag.next_tag();
                let cmd = format!("{} {}\r\n", tag, cmd);
                match server.command(&cmd) {
                    Ok(_) => Ok(IMAPClient::Logout),
                    Err(e) => Err((IMAPClient::Authenticated(server), e)),
                }
            }
            IMAPClient::Selected(mut mailbox) => {
                let tag = mailbox.tag.next_tag();
                let cmd = format!("{} {}\r\n", tag, cmd);
                match mailbox.command(&cmd) {
                    Ok(_) => Ok(IMAPClient::Logout),
                    Err(e) => Err((IMAPClient::Selected(mailbox), e)),
                }
            }
            IMAPClient::Logout => {
                Ok(IMAPClient::Logout)
            }
        }
    }

    fn read_greeting<T: Read + Write>(stream: &mut T) -> Result<String, IMAPError> {

        let mut buf = String::new();
        let _ = stream.read_to_string(&mut buf);
        // try!(check_response());
        Ok(buf)
    }

    fn check_response(response: String) -> Result<String, IMAPError> {
        if response.len() < 4 {return Err(IMAPError::Invalid(response))}
        let view: &[u8] = &response.as_bytes()[0..4];

        match view {
            b"* OK" => return Ok(response.to_owned()),
            b"* NO" => return Err(IMAPError::No(response.to_owned())),
            b"* BA" => return Err(IMAPError::Bad(response.to_owned())),
            _ => return Err(IMAPError::Invalid(response.to_owned())),
        }
    }

    fn check_tagged_response(response: String, tag: &str) -> Result<String, IMAPError> {
        if response.len() < tag.len() {return Err(IMAPError::Invalid(response))}
        Ok(response)
        let view: &[u8] = &response.as_bytes()[0..tag.len()];

        if view == tag.as_bytes() {
            let view: &[u8] = &response.as_bytes()[tag.len()..tag.len() + 3];

            match view {
                b" OK" => Ok(response.to_owned()),
                b" NO" => Err(IMAPError::No(response.to_owned())),
                b" BA" => Err(IMAPError::Bad(response.to_owned())),
                _ => Err(IMAPError::Invalid(response.to_owned())),
            }
        } else {
            Err(IMAPError::Invalid(response.to_owned()))
        }
    }
}

impl Mailbox {



    // fn CHECK() -> TypeName {
    // unimplemented!()
    // }
    //
    // fn CLOSE() -> TypeName {
    // unimplemented!()
    // }
    //
    // fn EXPUNGE() -> TypeName {
    // unimplemented!()
    // }
    //
    // fn SEARCH() -> TypeName {
    // unimplemented!()
    // }

    pub fn fetch(&mut self, sequence_set: SequenceSet) -> Result<String, IMAPError> {

        let args = format!("{}:{}",
                           sequence_set.0.to_string(),
                           sequence_set.1.to_string());

        let tag = self.tag.next_tag();
        let cmd = format!("{} FETCH {}\r\n", tag, args);

        match self.command(&cmd) {
            Ok(r) => IMAPClient::check_tagged_response(r, &tag),
            Err(e) => Err(e),
        }
    }

    // fn STORE() -> TypeName {
    // unimplemented!()
    // }
    //
    // fn COPY() -> TypeName {
    // unimplemented!()
    // }
    //
    // fn UID() -> TypeName {
    // unimplemented!()
    // }


    fn command(&mut self, cmd: &str) -> Result<String, IMAPError> {
        match &mut self.imap {
            &mut IMAPConnection::Basic(ref mut stream) => {
                let _ = stream.write(cmd.as_bytes());
                let mut buf = String::new();
                let _ = stream.read_to_string(&mut buf);
                println!("{}", buf);
                Ok(buf)
            }
            &mut IMAPConnection::Ssl(ref mut stream) => {
                let _ = stream.write(cmd.as_bytes());
                let mut buf = String::new();
                let _ = stream.read_to_string(&mut buf);
                println!("{}", buf);
                Ok(buf)
            }
            &mut IMAPConnection::Disconnected =>
                Err(IMAPError::LoginError("Not connected to server.".to_owned())),
        }
    }
}

impl MailServer {
    fn command(&mut self, cmd: &str) -> Result<String, IMAPError> {
        match &mut self.imap {
            &mut IMAPConnection::Basic(ref mut stream) => {
                let _ = stream.write(cmd.as_bytes());
                let mut buf = String::new();
                let _ = stream.read_to_string(&mut buf);
                println!("{}", buf);
                Ok(buf)
            }
            &mut IMAPConnection::Ssl(ref mut stream) => {
                let _ = stream.write(cmd.as_bytes());
                let mut buf = String::new();
                let _ = stream.read_to_string(&mut buf);
                println!("{}", buf);
                Ok(buf)
            }
            &mut IMAPConnection::Disconnected =>
                Err(IMAPError::LoginError("Not connected to server.".to_owned())),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
// }
