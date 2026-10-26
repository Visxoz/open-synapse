//! Device arrival and removal, from I/O Kit's matching notifications, on a
//! run loop of their own.

use std::{
    ffi::c_void,
    io, ptr,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

use core_foundation::{
    base::TCFType,
    runloop::{CFRunLoop, CFRunLoopSource, kCFRunLoopDefaultMode},
};
use io_kit_sys::{
    IONotificationPortCreate, IONotificationPortDestroy, IONotificationPortGetRunLoopSource, IONotificationPortRef,
    IOServiceAddMatchingNotification,
    keys::{kIOFirstMatchNotification, kIOTerminatedNotification},
    types::io_iterator_t,
};
use razer_protocol::VENDOR_ID;

use super::hid::{MAIN_PORT, Object, matching};

type Callback = Box<dyn Fn() + Send + Sync>;

/// Calls back whenever a Razer HID interface comes or goes, from a thread of
/// its own, until dropped.
pub struct Watcher {
    run_loop: CFRunLoop,
    thread: Option<JoinHandle<()>>,
}

impl Watcher {
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let (started, start) = mpsc::sync_channel(1);
        let thread = thread::Builder::new().name("hot-plug".into()).spawn(move || {
            let callback: Callback = Box::new(on_change);
            match Notifications::new(&callback) {
                Ok(notifications) => {
                    let _ = started.send(Ok(CFRunLoop::get_current()));
                    CFRunLoop::run_current();
                    drop(notifications);
                }
                Err(e) => drop(started.send(Err(e))),
            }
        })?;
        let run_loop = start.recv().map_err(io::Error::other)??;
        Ok(Self {
            run_loop,
            thread: Some(thread),
        })
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let Some(thread) = self.thread.take() else { return };
        // A run loop not yet running ignores a stop, so it is repeated.
        while !thread.is_finished() {
            self.run_loop.stop();
            thread::sleep(Duration::from_millis(1));
        }
        let _ = thread.join();
    }
}

/// I/O Kit's notifications of Razer's HID interfaces coming and going,
/// delivered to this thread's run loop until dropped.
struct Notifications {
    port: IONotificationPortRef,
    iterators: Vec<Object>,
}

impl Notifications {
    /// `callback` must outlive the notifications.
    fn new(callback: &Callback) -> io::Result<Self> {
        // SAFETY: plain arguments; a null port is checked.
        let port = unsafe { IONotificationPortCreate(MAIN_PORT) };
        if port.is_null() {
            return Err(io::Error::other("no I/O Kit notification port"));
        }
        let mut notifications = Self {
            port,
            iterators: Vec::with_capacity(2),
        };
        // SAFETY: the source belongs to the live port, and the mode is a
        // constant string.
        unsafe {
            let source = CFRunLoopSource::wrap_under_get_rule(IONotificationPortGetRunLoopSource(port));
            CFRunLoop::get_current().add_source(&source, kCFRunLoopDefaultMode);
        }
        for kind in [kIOFirstMatchNotification, kIOTerminatedNotification] {
            let dict = matching(VENDOR_ID)?;
            let mut iterator = 0;
            // SAFETY: the call consumes `dict`; the port, and so the
            // notification, ends before `callback` does.
            let status = unsafe {
                IOServiceAddMatchingNotification(
                    port,
                    kind,
                    dict.cast_const(),
                    changed,
                    ptr::from_ref(callback).cast_mut().cast(),
                    &mut iterator,
                )
            };
            if status != 0 {
                return Err(io::Error::other(format!("no I/O Kit notifications: {status:#x}")));
            }
            // Emptying the iterator arms the notification, passing over the
            // interfaces already present.
            drain(iterator);
            notifications.iterators.push(Object(iterator));
        }
        Ok(notifications)
    }
}

impl Drop for Notifications {
    fn drop(&mut self) {
        // SAFETY: the port came from IONotificationPortCreate and is destroyed
        // once, ending its notifications; the iterators are released after.
        unsafe { IONotificationPortDestroy(self.port) };
    }
}

/// Releases the objects waiting in `iterator`, which rearms its
/// notification; whether there were any.
fn drain(iterator: io_iterator_t) -> bool {
    let mut any = false;
    while Object::next(iterator).is_some() {
        any = true;
    }
    any
}

unsafe extern "C" fn changed(context: *mut c_void, iterator: io_iterator_t) {
    if drain(iterator) {
        // SAFETY: `context` is the callback given to `Notifications::new`,
        // which outlives the port delivering this.
        unsafe { (*context.cast::<Callback>())() };
    }
}
