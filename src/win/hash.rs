//! SHA-256 through Windows' own cryptography (CNG).

use std::io;

use windows_sys::Win32::Security::Cryptography::{BCRYPT_SHA256_ALG_HANDLE, BCryptHash};

/// The SHA-256 digest of `data`, in lower-case hex.
pub fn sha256(data: &[u8]) -> io::Result<String> {
    let len = u32::try_from(data.len()).map_err(|_| io::Error::other("too large to hash"))?;
    let mut digest = [0u8; 32];
    // SAFETY: the algorithm's pseudo-handle needs no opening, and `data` and
    // `digest` are valid for the lengths given.
    let status = unsafe {
        BCryptHash(
            BCRYPT_SHA256_ALG_HANDLE,
            std::ptr::null(),
            0,
            data.as_ptr(),
            len,
            digest.as_mut_ptr(),
            digest.len() as u32,
        )
    };
    if status != 0 {
        return Err(io::Error::other(format!("hashing failed: {status:#x}")));
    }
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_standard_vectors() {
        assert_eq!(
            sha256(b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256(b"").unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
