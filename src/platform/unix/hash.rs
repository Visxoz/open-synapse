//! SHA-256 through ring, which rustls brings for HTTPS anyway.

use std::io;

use ring::digest::{SHA256, digest};

/// The SHA-256 digest of `data`.
pub fn sha256(data: &[u8]) -> io::Result<[u8; 32]> {
    digest(&SHA256, data)
        .as_ref()
        .try_into()
        .map_err(|_| io::Error::other("unexpected digest length"))
}
