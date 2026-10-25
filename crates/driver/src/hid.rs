//! Windows HID: listing interfaces and exchanging feature reports.

use std::{io, ptr};

use windows_sys::Win32::{
    Devices::{
        DeviceAndDriverInstallation::{
            CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CM_Get_Device_Interface_List_SizeW, CM_Get_Device_Interface_ListW,
            CR_BUFFER_SMALL, CR_SUCCESS,
        },
        HumanInterfaceDevice::{
            HIDP_CAPS, HIDP_STATUS_SUCCESS, HidD_FreePreparsedData, HidD_GetFeature, HidD_GetHidGuid,
            HidD_GetPreparsedData, HidD_SetFeature, HidP_GetCaps,
        },
    },
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING},
};
use windows_sys::core::GUID;

/// A HID collection's device path, NUL-terminated, and its USB IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub path: Vec<u16>,
    pub vid: u16,
    pub pid: u16,
}

impl Interface {
    pub fn display_path(&self) -> String {
        String::from_utf16_lossy(self.path.strip_suffix(&[0]).unwrap_or(&self.path))
    }
}

pub(crate) fn hid_guid() -> GUID {
    let mut guid = GUID::from_u128(0);
    // SAFETY: `guid` is a valid out pointer for the duration of the call.
    unsafe { HidD_GetHidGuid(&mut guid) };
    guid
}

/// Every present HID collection whose vendor is `vid`.
pub fn interfaces(vid: u16) -> Vec<Interface> {
    let guid = hid_guid();
    let mut buf: Vec<u16> = Vec::new();
    loop {
        let mut len = 0;
        // SAFETY: `len` and `guid` are valid; a null device ID lists all.
        let cr = unsafe {
            CM_Get_Device_Interface_List_SizeW(&mut len, &guid, ptr::null(), CM_GET_DEVICE_INTERFACE_LIST_PRESENT)
        };
        if cr != CR_SUCCESS {
            return Vec::new();
        }
        buf.resize(len as usize, 0);
        // SAFETY: `buf` holds `len` u16s, as the call is told.
        let cr = unsafe {
            CM_Get_Device_Interface_ListW(
                &guid,
                ptr::null(),
                buf.as_mut_ptr(),
                len,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        match cr {
            CR_SUCCESS => break,
            // A device arrived between the two calls.
            CR_BUFFER_SMALL => continue,
            _ => return Vec::new(),
        }
    }
    buf.split(|&c| c == 0)
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            let (v, pid) = usb_ids(p)?;
            (v == vid).then(|| Interface {
                path: p.iter().copied().chain([0]).collect(),
                vid: v,
                pid,
            })
        })
        .collect()
}

/// The `VID_xxxx` and `PID_xxxx` in a device path.
pub(crate) fn usb_ids(path: &[u16]) -> Option<(u16, u16)> {
    let path = String::from_utf16_lossy(path).to_ascii_lowercase();
    let hex = |key: &str| {
        let at = path.find(key)? + key.len();
        u16::from_str_radix(path.get(at..at + 4)?, 16).ok()
    };
    Some((hex("vid_")?, hex("pid_")?))
}

/// An open HID collection.
pub struct Hid {
    handle: HANDLE,
    pub feature_len: usize,
}

// SAFETY: a file handle may be used from any thread; `Hid` is only ever
// used by one thread at a time, as it is not `Sync`.
unsafe impl Send for Hid {}

impl Hid {
    /// Opens without read or write access, which feature reports do not
    /// need, so collections Windows keeps for itself, such as the mouse's
    /// own, still open.
    pub fn open(path: &[u16]) -> io::Result<Self> {
        debug_assert_eq!(path.last(), Some(&0));
        // SAFETY: `path` is NUL-terminated; the other arguments are plain values.
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let mut hid = Self { handle, feature_len: 0 };
        hid.feature_len = hid.caps()?.FeatureReportByteLength.into();
        Ok(hid)
    }

    fn caps(&self) -> io::Result<HIDP_CAPS> {
        let mut data = 0;
        // SAFETY: `self.handle` is open and `data` is a valid out pointer.
        if !unsafe { HidD_GetPreparsedData(self.handle, &mut data) } {
            return Err(io::Error::last_os_error());
        }
        let mut caps = HIDP_CAPS::default();
        // SAFETY: `data` came from HidD_GetPreparsedData and is freed once, after use.
        let status = unsafe {
            let status = HidP_GetCaps(data, &mut caps);
            HidD_FreePreparsedData(data);
            status
        };
        if status != HIDP_STATUS_SUCCESS {
            return Err(io::Error::other("no HID capabilities"));
        }
        Ok(caps)
    }

    pub fn set_feature(&self, report: &[u8]) -> io::Result<()> {
        // SAFETY: the buffer is valid for reads of its length.
        let ok = unsafe { HidD_SetFeature(self.handle, report.as_ptr().cast(), report.len() as u32) };
        if ok { Ok(()) } else { Err(io::Error::last_os_error()) }
    }

    /// Reads the feature report whose ID is in `report[0]`.
    pub fn get_feature(&self, report: &mut [u8]) -> io::Result<()> {
        // SAFETY: the buffer is valid for writes of its length.
        let ok = unsafe { HidD_GetFeature(self.handle, report.as_mut_ptr().cast(), report.len() as u32) };
        if ok { Ok(()) } else { Err(io::Error::last_os_error()) }
    }
}

impl Drop for Hid {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateFileW and is closed once.
        unsafe { CloseHandle(self.handle) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ids_from_a_path() {
        let path: Vec<u16> =
            r"\\?\HID#VID_1532&PID_00E6&MI_03#8&35a3bb52&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
                .encode_utf16()
                .collect();
        assert_eq!(usb_ids(&path), Some((0x1532, 0x00E6)));
    }
}
