//! HTTPS downloads through WinHTTP, which Windows already has.

use std::{ffi::c_void, io, ptr};

use windows_sys::Win32::Networking::WinHttp::{
    URL_COMPONENTS, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE, WINHTTP_INTERNET_SCHEME_HTTPS,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect, WinHttpCrackUrl,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts,
};

use super::wide;

const TIMEOUT_MS: i32 = 15_000;

/// Closes a WinHTTP handle when dropped.
struct Handle(*mut c_void);

impl Handle {
    fn new(raw: *mut c_void) -> io::Result<Self> {
        if raw.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from WinHTTP and is closed once.
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn check(ok: i32) -> io::Result<()> {
    if ok != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// The body of `url`, which must be HTTPS, up to `limit` bytes.
pub fn get(url: &str, limit: usize) -> io::Result<Vec<u8>> {
    let url_w = wide(url);
    let mut parts = URL_COMPONENTS {
        dwStructSize: size_of::<URL_COMPONENTS>() as u32,
        // Non-zero lengths ask for pointers into `url_w`.
        dwHostNameLength: u32::MAX,
        dwUrlPathLength: u32::MAX,
        dwExtraInfoLength: u32::MAX,
        ..Default::default()
    };
    // SAFETY: `url_w` is NUL-terminated and outlives `parts`, which points into it.
    check(unsafe { WinHttpCrackUrl(url_w.as_ptr(), 0, 0, &mut parts) })?;
    if parts.nScheme != WINHTTP_INTERNET_SCHEME_HTTPS {
        return Err(io::Error::other("only HTTPS is allowed"));
    }
    // SAFETY: the cracked pointers and lengths lie within `url_w`.
    let (host, path) = unsafe {
        let host = std::slice::from_raw_parts(parts.lpszHostName, parts.dwHostNameLength as usize);
        let path = std::slice::from_raw_parts(
            parts.lpszUrlPath,
            (parts.dwUrlPathLength + parts.dwExtraInfoLength) as usize,
        );
        (String::from_utf16_lossy(host), String::from_utf16_lossy(path))
    };

    let agent = wide(concat!("RazerNative/", env!("CARGO_PKG_VERSION")));
    // SAFETY: `agent` is NUL-terminated; null proxy names use the system's.
    let session = Handle::new(unsafe {
        WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        )
    })?;
    // SAFETY: `session` is open.
    check(unsafe { WinHttpSetTimeouts(session.0, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS) })?;
    let host_w = wide(&host);
    // SAFETY: `session` is open and `host_w` NUL-terminated.
    let connection = Handle::new(unsafe { WinHttpConnect(session.0, host_w.as_ptr(), parts.nPort, 0) })?;
    let (verb, path_w) = (wide("GET"), wide(&path));
    // SAFETY: `connection` is open and the strings NUL-terminated; nulls take defaults.
    let request = Handle::new(unsafe {
        WinHttpOpenRequest(
            connection.0,
            verb.as_ptr(),
            path_w.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            WINHTTP_FLAG_SECURE,
        )
    })?;
    // SAFETY: `request` is open; no extra headers or body.
    check(unsafe { WinHttpSendRequest(request.0, ptr::null(), 0, ptr::null(), 0, 0, 0) })?;
    // SAFETY: `request` has been sent.
    check(unsafe { WinHttpReceiveResponse(request.0, ptr::null_mut()) })?;

    let (mut status, mut size) = (0u32, 4u32);
    // SAFETY: `status` holds the four bytes asked for.
    check(unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            (&mut status as *mut u32).cast(),
            &mut size,
            ptr::null_mut(),
        )
    })?;
    if status != 200 {
        return Err(io::Error::other(format!("HTTP {status}")));
    }

    let mut body = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let mut read = 0u32;
        // SAFETY: `chunk` is valid for writes of its length.
        check(unsafe { WinHttpReadData(request.0, chunk.as_mut_ptr().cast(), chunk.len() as u32, &mut read) })?;
        if read == 0 {
            return Ok(body);
        }
        body.extend_from_slice(&chunk[..read as usize]);
        if body.len() > limit {
            return Err(io::Error::other("response too large"));
        }
    }
}
