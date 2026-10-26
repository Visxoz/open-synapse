//! What Linux desktops share: the XDG folders, and X11 for the program in
//! front, as Wayland keeps that from other programs. Games under Wine and
//! Proton draw through X11 even on Wayland, so they are seen.

use std::{fs, path::PathBuf};

use x11rb::{
    connection::Connection,
    protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window},
    rust_connection::RustConnection,
};

use super::home;
use crate::platform::{Fonts, file_name_lower};

pub const FONTS: Option<Fonts> = Some(Fonts {
    display: "",
    body: "",
    mono: "DejaVu Sans Mono",
});

/// Parts of the desktop that list windows of their own.
pub const SYSTEM: &[&str] = &["gnome-shell", "plasmashell", "xfce4-panel", "xfdesktop"];

/// `$XDG_CONFIG_HOME`, else `~/.config`.
pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| Some(home()?.join(".config")))
}

/// Starting at sign-in is an XDG autostart entry.
pub fn autostart_file() -> Option<PathBuf> {
    Some(config_dir()?.join("autostart/open-synapse.desktop"))
}

pub fn autostart_entry(exe: &str, arg: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Open Synapse\nExec={} {arg}\nIcon=open-synapse\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        quote(exe)
    )
}

/// `arg` as a desktop entry's `Exec` key needs it: quoted, with `"`, `` ` ``,
/// `$` and `\` escaped, then every backslash escaped again, as the key is a
/// string, and `%` doubled, as it starts a field code.
fn quote(arg: &str) -> String {
    let mut quoted = String::from("\"");
    for c in arg.chars() {
        match c {
            '"' | '`' | '$' => {
                quoted.push_str(r"\\");
                quoted.push(c);
            }
            '\\' => quoted.push_str(r"\\\\"),
            '%' => quoted.push_str("%%"),
            _ => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

/// The X server, with the atoms that say which windows are whose.
pub struct Display {
    connection: RustConnection,
    root: Window,
    active: Atom,
    clients: Atom,
    pid: Atom,
}

impl Display {
    /// None without X11, as on Wayland with no XWayland.
    pub fn open() -> Option<Self> {
        let (connection, screen) = x11rb::connect(None).ok()?;
        let root = connection.setup().roots.get(screen)?.root;
        let names: [&[u8]; 3] = [b"_NET_ACTIVE_WINDOW", b"_NET_CLIENT_LIST", b"_NET_WM_PID"];
        let cookies: Vec<_> = names
            .iter()
            .map(|name| connection.intern_atom(false, name).ok())
            .collect::<Option<_>>()?;
        let atoms: Vec<Atom> = cookies
            .into_iter()
            .map(|c| Some(c.reply().ok()?.atom))
            .collect::<Option<_>>()?;
        let &[active, clients, pid] = atoms.as_slice() else {
            return None;
        };
        Some(Self {
            connection,
            root,
            active,
            clients,
            pid,
        })
    }

    /// Up to `len` 32-bit values of a window's property.
    fn property(&self, window: Window, property: Atom, kind: AtomEnum, len: u32) -> Vec<u32> {
        self.connection
            .get_property(false, window, property, kind, 0, len)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| Some(r.value32()?.collect()))
            .unwrap_or_default()
    }

    /// The window in front, unless X11 cannot say, as when a Wayland
    /// window is.
    pub fn front(&self) -> Option<u32> {
        let window = *self.property(self.root, self.active, AtomEnum::WINDOW, 1).first()?;
        (window != 0).then_some(window)
    }

    /// The program that owns `window`.
    pub fn name(&self, window: u32) -> Option<String> {
        program(*self.property(window, self.pid, AtomEnum::CARDINAL, 1).first()?)
    }

    /// The programs that own a window, in no order.
    pub fn programs(&self) -> Vec<String> {
        let windows = self.property(self.root, self.clients, AtomEnum::WINDOW, 4096);
        // Every request goes out before any reply is awaited.
        let cookies: Vec<_> = windows
            .iter()
            .filter_map(|&w| {
                self.connection
                    .get_property(false, w, self.pid, AtomEnum::CARDINAL, 0, 1)
                    .ok()
            })
            .collect();
        cookies
            .into_iter()
            .filter_map(|c| c.reply().ok()?.value32()?.next())
            .filter_map(program)
            .collect()
    }
}

/// A process's lower-case program name: the file name its command line
/// starts with, which for a Windows game under Wine or Proton is its exe,
/// else its executable's.
fn program(pid: u32) -> Option<String> {
    let cmdline = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let first = cmdline.split(|&b| b == 0).next().unwrap_or_default();
    let name = String::from_utf8_lossy(first);
    match name.rsplit(['/', '\\']).next() {
        Some(name) if !name.is_empty() => Some(name.to_lowercase()),
        _ => file_name_lower(&fs::read_link(format!("/proc/{pid}/exe")).ok()?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_an_exec_key() {
        assert_eq!(
            quote("/opt/Open Synapse/open-synapse"),
            r#""/opt/Open Synapse/open-synapse""#
        );
        assert_eq!(quote(r#"/a"b$c\d%e"#), r#""/a\\"b\\$c\\\\d%%e""#);
    }

    #[test]
    fn names_this_process() {
        assert_eq!(program(std::process::id()), crate::platform::own_exe());
    }
}
