//! Listing HID interfaces in the I/O Registry and exchanging feature
//! reports through IOHIDDevice.

use std::{io, ptr};

use core_foundation::{
    base::{CFIndex, CFRelease, CFType, TCFType},
    data::CFData,
    dictionary::{CFDictionarySetValue, CFMutableDictionaryRef},
    number::CFNumber,
    string::CFString,
};
use io_kit_sys::{
    IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperty, IORegistryEntryGetRegistryEntryID,
    IORegistryEntryIDMatching, IOServiceGetMatchingService, IOServiceGetMatchingServices, IOServiceMatching,
    hid::{
        base::IOHIDDeviceRef,
        device::{IOHIDDeviceClose, IOHIDDeviceCreate, IOHIDDeviceGetReport, IOHIDDeviceOpen, IOHIDDeviceSetReport},
        keys::{kIOHIDOptionsTypeNone, kIOHIDReportTypeFeature},
    },
    ret::{IOReturn, kIOReturnNotPermitted, kIOReturnSuccess},
    types::{io_iterator_t, io_object_t},
};

use crate::{Interface, descriptor};

/// `kIOMainPortDefault`, which is `MACH_PORT_NULL`.
pub(super) const MAIN_PORT: u32 = 0;

/// An interface's registry entry ID, which, unlike a path, is unique.
pub type Path = u64;

pub fn display(id: &Path) -> String {
    format!("DevSrvsID:{id}")
}

fn check(status: IOReturn) -> io::Result<()> {
    if status == kIOReturnSuccess {
        Ok(())
    } else if status == kIOReturnNotPermitted {
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "not permitted"))
    } else {
        Err(io::Error::other(format!("I/O Kit error {status:#x}")))
    }
}

/// An I/O Kit object, released when dropped.
pub(super) struct Object(pub io_object_t);

impl Drop for Object {
    fn drop(&mut self) {
        // SAFETY: the object is owned, and released once.
        unsafe { IOObjectRelease(self.0) };
    }
}

impl Object {
    /// The next object from `iterator`, which must be live.
    pub(super) fn next(iterator: io_iterator_t) -> Option<Self> {
        // SAFETY: the caller keeps `iterator` live; 0 marks its end.
        let object = unsafe { IOIteratorNext(iterator) };
        (object != 0).then_some(Self(object))
    }

    fn property(&self, key: &'static str) -> Option<CFType> {
        let key = CFString::from_static_string(key);
        // SAFETY: the entry is live; a null allocator is the default one.
        let value = unsafe { IORegistryEntryCreateCFProperty(self.0, key.as_concrete_TypeRef(), ptr::null(), 0) };
        // SAFETY: a non-null result is a CF object handed to us.
        (!value.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(value) })
    }
}

/// A dictionary matching the HID interfaces of vendor `vid`, for the I/O
/// Kit calls that consume one.
pub(super) fn matching(vid: u16) -> io::Result<CFMutableDictionaryRef> {
    // SAFETY: the class name is NUL-terminated; a null result is checked.
    let dict = unsafe { IOServiceMatching(c"IOHIDDevice".as_ptr()) };
    if dict.is_null() {
        return Err(io::Error::other("no matching dictionary"));
    }
    let (key, value) = (CFString::from_static_string("VendorID"), CFNumber::from(i32::from(vid)));
    // SAFETY: `dict` holds CF objects and retains both.
    unsafe { CFDictionarySetValue(dict, key.as_CFTypeRef(), value.as_CFTypeRef()) };
    Ok(dict)
}

/// Every HID interface whose vendor is `vid`.
pub fn interfaces(vid: u16) -> Vec<Interface> {
    let Ok(dict) = matching(vid) else {
        return Vec::new();
    };
    let mut iterator = 0;
    // SAFETY: the call consumes `dict` and writes the iterator.
    let status = unsafe { IOServiceGetMatchingServices(MAIN_PORT, dict.cast_const(), &mut iterator) };
    if status != kIOReturnSuccess {
        log::debug!("cannot list HID services: {status:#x}");
        return Vec::new();
    }
    let iterator = Object(iterator);
    std::iter::from_fn(|| Object::next(iterator.0))
        .filter_map(|service| {
            let pid = service.property("ProductID")?.downcast::<CFNumber>()?.to_i32()?;
            let mut id = 0;
            // SAFETY: the entry is live and `id` a valid out pointer.
            let found = unsafe { IORegistryEntryGetRegistryEntryID(service.0, &mut id) } == kIOReturnSuccess;
            found.then_some(Interface {
                path: id,
                pid: u16::try_from(pid).ok()?,
            })
        })
        .collect()
}

/// An IOHIDDevice, released when dropped.
struct Device(IOHIDDeviceRef);

impl Drop for Device {
    fn drop(&mut self) {
        // SAFETY: the device came from IOHIDDeviceCreate and is released once.
        unsafe { CFRelease(self.0.cast()) };
    }
}

/// An open HID interface.
pub struct Hid {
    device: Device,
}

// SAFETY: an IOHIDDevice may be used from any thread; `Hid` is only ever
// used by one thread at a time, as it is not `Sync`.
unsafe impl Send for Hid {}

impl Hid {
    /// Opens the interface if its longest feature report is `feature_len`
    /// bytes with the report ID, read from its descriptor beforehand so that
    /// no other interface is opened, which macOS may ask the user about.
    pub fn open(id: &Path, feature_len: usize) -> io::Result<Option<Self>> {
        // SAFETY: the lookup consumes the matching dictionary.
        let service = unsafe { IOServiceGetMatchingService(MAIN_PORT, IORegistryEntryIDMatching(*id).cast_const()) };
        if service == 0 {
            return Err(io::ErrorKind::NotFound.into());
        }
        let service = Object(service);
        let descriptor = service
            .property("ReportDescriptor")
            .and_then(|d| d.downcast::<CFData>());
        if descriptor.is_none_or(|d| descriptor::feature_len(d.bytes()) != feature_len) {
            return Ok(None);
        }
        // SAFETY: the service is live; a null allocator is the default one.
        let device = unsafe { IOHIDDeviceCreate(ptr::null(), service.0) };
        if device.is_null() {
            return Err(io::Error::other("not a HID device"));
        }
        let device = Device(device);
        // SAFETY: the device is live.
        check(unsafe { IOHIDDeviceOpen(device.0, kIOHIDOptionsTypeNone) })?;
        Ok(Some(Self { device }))
    }

    pub fn set_feature(&self, report: &[u8]) -> io::Result<()> {
        let (id, skip) = report_id(report);
        let data = &report[skip..];
        // SAFETY: the device is open and `data` valid for reads of its length.
        check(unsafe {
            IOHIDDeviceSetReport(
                self.device.0,
                kIOHIDReportTypeFeature,
                id,
                data.as_ptr(),
                data.len() as CFIndex,
            )
        })
    }

    /// Reads the feature report whose ID is in `report[0]`.
    pub fn get_feature(&self, report: &mut [u8]) -> io::Result<()> {
        let (id, skip) = report_id(report);
        let data = &mut report[skip..];
        let mut len = data.len() as CFIndex;
        // SAFETY: the device is open and `data` valid for writes of `len` bytes.
        check(unsafe { IOHIDDeviceGetReport(self.device.0, kIOHIDReportTypeFeature, id, data.as_mut_ptr(), &mut len) })
    }
}

/// A report's ID, and how many bytes to skip for I/O Kit, which leaves out
/// an ID of 0.
fn report_id(report: &[u8]) -> (CFIndex, usize) {
    match report.first() {
        Some(0) => (0, 1),
        Some(&id) => (CFIndex::from(id), 0),
        None => (0, 0),
    }
}

impl Drop for Hid {
    fn drop(&mut self) {
        // SAFETY: the device was opened in `open`; it is released after this.
        unsafe { IOHIDDeviceClose(self.device.0, kIOHIDOptionsTypeNone) };
    }
}
