#![allow(unused_variables)]
extern crate imap;

use imap::{IMAPConnection, IMAPClient};

fn main() {

    let con = IMAPConnection::new_tls("outlook.office365.com", 993).unwrap();

    let client = IMAPClient::connect(con).unwrap();

    // After 'login' command, the IMAPClient can only be Authenticated (success) or UnAuthenticated
    // (upon error). The original client is consumed and a new one, in the new state, is returned.
    let client = match client.login("username@email.com", "passwd") {
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
    if let IMAPClient::Authenticated(_) = client {
        let mut client = client.select("INBOX").unwrap();

        // Once we are in the Selected state we can access more commands through the 'Mailbox' struct
        if let &mut IMAPClient::Selected(ref mut inbox) = &mut client {
            // fetch email '3'
            let emails = inbox.fetch(3).unwrap();
            println!("Fetched {} emails", emails.len());

            // fetch emails 1 to 100
            let emails = inbox.fetch((1,100)).unwrap();
            println!("Fetched {} emails", emails.len());
            // let mut client = client.logout().unwrap();

            // for email in emails {
            //     println!("{:#?}", email);
            // }

        }
        let client = client.logout().unwrap();


        if let IMAPClient::Logout = client {
            println!("Logged out of server - this client is no longer usable.");
        } else {
            unreachable!()
        }
    }


    // if let &mut IMAPClient::Selected(ref mut mailbox) = &mut client {
    //     let email = mailbox.fetch((0,2)).unwrap();
    //     println!("{}", email);
    // }

}
