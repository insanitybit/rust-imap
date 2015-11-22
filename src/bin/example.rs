#![allow(unused_variables)]
extern crate imap;

use imap::{IMAPConnection, IMAPClient};

fn main() {

    let con = IMAPConnection::new_tls("outlook.office365.com", 993).unwrap();

    let client = IMAPClient::connect(con).unwrap();

    // After 'login' command, the IMAPClient can only be Authenticated (success) or UnAuthenticated
    // (upon error). The original client is consumed and a new one, in the new state, is returned.
    let client = match client.login("user", "password") {
        Ok(client)  => client,
        Err((client, e))  => {
            match client {
                // Errors never change the state of the client
                IMAPClient::UnAuthenticated(_)  => println!("We failed to authenticate :( {}", e),
                _   => unreachable!("We can very easily reason about the state we are in")
            }
            client
        }
    };
    println!("{:#?}", client);

    // If we're authenticated we can select.
    if let &IMAPClient::Authenticated(_) = &client {
        let client = client.select("INBOX").unwrap();

        // Once we are in the Selected state we can access more commands through the 'Mailbox' struct
        if let IMAPClient::Selected(mut inbox) = client {
            // Grab the first 3 emails
            let _ = inbox.fetch((0,2));
        }
    }

    // if let &mut IMAPClient::Selected(ref mut mailbox) = &mut client {
    //     let email = mailbox.fetch((0,2)).unwrap();
    //     println!("{}", email);
    // }

}
