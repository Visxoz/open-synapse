//! Device arrival and removal, from the configuration manager.

use std::{ffi::c_void, io, mem, ptr};

use razer_protocol::VENDOR_ID;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    CM_NOTIFY_ACTION, CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL, CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL,
    CM_NOTIFY_EVENT_DATA, CM_NOTIFY_FILTER, CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE, CM_Register_Notification,
    CM_Unregister_Notification, CR_SUCCESS, HCMNOTIFICATION,
};

use crate::hid::{hid_guid, usb_ids};

type Callback = Box<dyn Fn() + Send + Sync>;

/// Calls back whenever a Razer HID collection comes or goes, from a
/// system thread, until dropped.
pub struct Watcher {
    handle: HCMNOTIFICATION,
    callback: *mut Callback,
}

// SAFETY: the handle is only used to unregister, and the callback is `Send + Sync`.
unsafe impl Send for Watcher {}

impl Watcher {
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let callback: *mut Callback = Box::into_raw(Box::new(Box::new(on_change)));
        let mut filter = CM_NOTIFY_FILTER {
            cbSize: mem::size_of::<CM_NOTIFY_FILTER>() as u32,
            FilterType: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
            ..Default::default()
        };
        filter.u.DeviceInterface.ClassGuid = hid_guid();
        let mut handle = ptr::null_mut();
        // SAFETY: `filter` is initialised, and `callback` stays valid until
        // the notification is unregistered in `drop`.
        let cr = unsafe { CM_Register_Notification(&filter, callback as *const c_void, Some(notify), &mut handle) };
        if cr != CR_SUCCESS {
            // SAFETY: registration failed, so nothing else holds `callback`.
            drop(unsafe { Box::from_raw(callback) });
            return Err(io::Error::other(format!("CM_Register_Notification failed: {cr}")));
        }
        Ok(Self { handle, callback })
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        // SAFETY: unregistering waits for running callbacks, after which
        // none can see `callback`, which was leaked from a box in `new`.
        unsafe {
            CM_Unregister_Notification(self.handle);
            drop(Box::from_raw(self.callback));
        }
    }
}

unsafe extern "system" fn notify(
    _: HCMNOTIFICATION,
    context: *const c_void,
    action: CM_NOTIFY_ACTION,
    data: *const CM_NOTIFY_EVENT_DATA,
    size: u32,
) -> u32 {
    if action != CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL && action != CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL {
        return 0;
    }
    // SAFETY: for interface events `data` holds `size` bytes ending in the
    // interface's NUL-terminated symbolic link.
    let link = unsafe {
        let start = ptr::addr_of!((*data).u.DeviceInterface.SymbolicLink).cast::<u16>();
        let max = (size as usize).saturating_sub(start as usize - data as usize) / 2;
        let link = std::slice::from_raw_parts(start, max);
        &link[..link.iter().position(|&c| c == 0).unwrap_or(max)]
    };
    if usb_ids(link).is_some_and(|(vid, _)| vid == VENDOR_ID) {
        // SAFETY: `context` is the callback registered in `Watcher::new`, alive until unregistering.
        unsafe { (*context.cast::<Callback>())() };
    }
    0
}
