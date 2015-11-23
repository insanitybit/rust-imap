#![allow(unused_variables, unused_imports, dead_code)]
extern crate openssl;
extern crate rand;
extern crate regex;

pub mod imaperror;

use imaperror::IMAPError;
use openssl::ssl::{SslContext, SslStream, SslMethod, Ssl};
use rand::Rng;
use regex::Regex;
use std::io::prelude;
use std::io::{Write, Read};
use std::net::TcpStream;
use std::time::Duration;

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
    flags: String,
    exists: String,
    recent: String,
    unseen: Option<String>,
    permanentflags: Option<String>,
    uidnext: Option<String>,
    uidvalidity: Option<String>,
    permission: Option<String>
}

#[derive(Debug)]
pub struct MailboxResponse {
    flags: String,
    exists: String,
    recent: String,
    unseen: Option<String>,
    permanentflags: Option<String>,
    uidnext: Option<String>,
    uidvalidity: Option<String>,
    permission: Option<String>
}

#[derive(Debug)]
pub struct Email;// {

// pub type SequenceSet = (u32, u32);


#[derive(Debug)]
pub enum Macro {
    All,
    Fast,
    Full,
}

pub enum DataItem {
    SequenceSet(u32,u32),
    Atom(u32),
    Macro
}

impl From<(u32, u32)> for DataItem {
    fn from(ss: (u32, u32)) -> DataItem {
        DataItem::SequenceSet(ss.0, ss.1)
    }
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
                        match IMAPClient::check_select_response(&res, &tag) {
                                Ok(mailres)   => {
                                    let mailbox = Mailbox {
                                        imap: server.imap,
                                        tag: server.tag,
                                        flags: mailres.flags,
                                        exists: mailres.exists,
                                        recent: mailres.recent,
                                        unseen: mailres.unseen,
                                        permanentflags: mailres.permanentflags,
                                        uidnext: mailres.uidnext,
                                        uidvalidity: mailres.uidvalidity,
                                        permission: mailres.permission
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
        let buf = try!(IMAPClient::check_response(buf));
        Ok(buf)
    }

    fn capture_response(response: &str, re: Regex) -> Result<String, IMAPError> {
        let cap = re.captures_iter(response).next();

        let exists = match cap {
            Some(cap)   => cap.at(1),
            None    => return Err(IMAPError::Invalid("Could not find required response command".to_owned()))
        };

        match exists {
            Some(value) => Ok(value.to_owned()),
            None    => Err(IMAPError::Invalid("Could not find required response for command".to_owned()))
        }
    }

    fn check_select_response(response: &str, tag: &str) -> Result<MailboxResponse, IMAPError> {
        let existsre = Regex::new(r"(\d+) EXISTS\r\n").unwrap();
        let recentre = Regex::new(r"(\d+) RECENT\r\n").unwrap();
        let flagsre = Regex::new(r"FLAGS \(([^\)]+)\)").unwrap();
        let unseenre = Regex::new(r"\* OK \[UNSEEN (\d+)\]").unwrap();
        let permanentflagsre = Regex::new(r"PERMANENTFLAGS \(([^\)]+)\)").unwrap();
        let uidnextre = Regex::new(r"\* OK \[UIDNEXT (\d+)\]").unwrap();
        let uidvalidityre = Regex::new(r"\* OK \[UIDVALIDITY (\d+)\]").unwrap();
        let permissionre = Regex::new(r" OK \[([^\]]+)\] SELECT").unwrap();

        let exists = try!(IMAPClient::capture_response(response, existsre));
        let recent = try!(IMAPClient::capture_response(response, recentre));
        let flags = try!(IMAPClient::capture_response(response, flagsre));

        let unseen = IMAPClient::capture_response(response, unseenre).ok();
        let permanentflags = IMAPClient::capture_response(response, permanentflagsre).ok();
        let uidnext = IMAPClient::capture_response(response, uidnextre).ok();
        let uidvalidity = IMAPClient::capture_response(response, uidvalidityre).ok();
        let mut permission = None;

        let tagged_ok = tag.to_owned() + " OK";

        if let Some(index) = response.find(&tagged_ok) {
            let view = &response[index+ tag.len()..];
            permission = IMAPClient::capture_response(view, permissionre).ok();
        }
        Ok(MailboxResponse {
            exists:exists,
            recent:recent,
            flags:flags,
            unseen:unseen,
            permanentflags:permanentflags,
            uidnext:uidnext,
            uidvalidity:uidvalidity,
            permission: permission
        })
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

    pub fn fetch<T: Into<DataItem>>(&mut self, data_item: T) -> Result<Vec<Email>, IMAPError> {
        let data_item = data_item.into();
        let args = match data_item {
            DataItem::SequenceSet(l, h) => format!("{}:{}", l.to_string(), h.to_string()),
            _                  => panic!()

        };


        let tag = self.tag.next_tag();
        let cmd = format!("{} FETCH {} ALL\r\n", tag, args);

        let response = try!(self.command(&cmd));
        // println!("{}", response);
        let response = try!(Mailbox::parse_fetch_response(&response));
        Ok(response)
    }

    fn parse_fetch_response<'a>(res: &'a str) -> Result<Vec<Email>, IMAPError> {
        let emailre = Regex::new(r"\* \d+ FETCH(.*)\r\n").unwrap();
        let mut emails = Vec::new();
        let captures = emailre.captures_iter(res);


        for cap in captures {
            if let Some(email) = cap.at(1) {
                emails.push(email);
            }
        }

        if emails.is_empty() {
            return Err(IMAPError::Invalid(res.to_owned()));
        }

        let emails = Mailbox::parse_emails(&emails);
        Ok(emails)
    }

    fn parse_emails(emails: &[&str]) -> Vec<Email> {
        unimplemented!();
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
                Ok(buf)
            }
            &mut IMAPConnection::Ssl(ref mut stream) => {
                let _ = stream.write(cmd.as_bytes());
                let mut buf = String::new();
                let _ = stream.read_to_string(&mut buf);
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
                Ok(buf)
            }
            &mut IMAPConnection::Ssl(ref mut stream) => {
                let _ = stream.write(cmd.as_bytes());
                let mut buf = String::new();
                let _ = stream.read_to_string(&mut buf);
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
