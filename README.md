rust-imap
================
IMAP Client for Rust

This is a fork of [this repo](https://github.com/mattnenterprise/rust-imap) with
some changes.

Currently I've made pushed few changes, notably:
* No longer using wildcards for dependency versions
* Remove panic when email is not valid utf8

### TODO:
* Move to openssl 0.7.0
* Implement an IMAPError type
* Remove any/ all raw unwrapping
* Documentation/ #![deny(missing_docs)]
* Expose client state, handle state according to IMAP spec

### License

MIT
