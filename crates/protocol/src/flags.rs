/// A `Copy` set of named bits, usable in `const` tables through [`set!`].
macro_rules! flags {
    ($(#[$meta:meta])* $name:ident: $ty:ty { $($(#[$fmeta:meta])* $flag:ident = $bit:expr),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
        pub struct $name(pub(crate) $ty);

        impl $name {
            pub const NONE: Self = Self(0);
            $($(#[$fmeta])* pub const $flag: Self = Self(1 << $bit);)+
            const ALL: &'static [Self] = &[$(Self::$flag),+];

            pub const fn contains(self, other: Self) -> bool {
                self.0 & other.0 == other.0 && other.0 != 0
            }

            /// The position of a single flag.
            pub const fn index(self) -> u32 {
                self.0.trailing_zeros()
            }

            pub const fn from_index(i: u32) -> Self {
                Self(1 << i)
            }

            pub fn iter(self) -> impl Iterator<Item = Self> {
                Self::ALL.iter().copied().filter(move |&f| self.contains(f))
            }
        }
    };
}

/// `set!(Effects: STATIC | OFF)`, a flag set built at compile time.
macro_rules! set {
    ($ty:ident: $($flag:ident)|+) => {
        $ty($($ty::$flag.0)|+)
    };
}

pub(crate) use {flags, set};
