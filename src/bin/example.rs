#![allow(unused_variables)]
extern crate imap;

use imap::{IMAPConnection, IMAPClient};

fn main() {

    let con = IMAPConnection::new_tls("imap.gmail.com", 993).unwrap();

    let client = IMAPClient::connect(con).unwrap();
    let client = client.login("thomasmcvane@gmail.com", "iamveryvain");
    println!("{:#?}", client);
    // let mut client = client.select("INBOX").unwrap();
    //
    // if let &mut IMAPClient::Selected(ref mut mailbox) = &mut client {
    //     let email = mailbox.fetch((0,2)).unwrap();
    //     println!("{}", email);
    // }

}
