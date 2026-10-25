/// Every request and response is 90 bytes: an 8-byte header, 80 bytes of
/// arguments, a checksum and a reserved byte.
pub const REPORT_LEN: usize = 90;
pub const ARGS_LEN: usize = 80;

/// Argument 0 of many commands: whether the device keeps the value.
pub const NOSTORE: u8 = 0x00;
pub const VARSTORE: u8 = 0x01;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    New,
    Busy,
    Ok,
    Failed,
    Timeout,
    Unsupported,
    Unknown(u8),
}

impl From<u8> for Status {
    fn from(b: u8) -> Self {
        match b {
            0 => Self::New,
            1 => Self::Busy,
            2 => Self::Ok,
            3 => Self::Failed,
            4 => Self::Timeout,
            5 => Self::Unsupported,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub status: Status,
    pub tid: u8,
    pub size: u8,
    pub class: u8,
    pub id: u8,
    pub args: [u8; ARGS_LEN],
}

impl Report {
    pub const BLANK: Self = Self {
        status: Status::New,
        tid: 0,
        size: 0,
        class: 0,
        id: 0,
        args: [0; ARGS_LEN],
    };

    pub fn new(class: u8, id: u8, size: u8, args: &[u8]) -> Self {
        let mut report = Self {
            status: Status::New,
            tid: 0xFF,
            size,
            class,
            id,
            args: [0; ARGS_LEN],
        };
        report.args[..args.len()].copy_from_slice(args);
        report
    }

    pub fn with_tid(mut self, tid: u8) -> Self {
        self.tid = tid;
        self
    }

    pub fn encode(&self) -> [u8; REPORT_LEN] {
        let mut b = [0; REPORT_LEN];
        b[1] = self.tid;
        b[5] = self.size;
        b[6] = self.class;
        b[7] = self.id;
        b[8..88].copy_from_slice(&self.args);
        b[88] = checksum(&b);
        b
    }

    pub fn decode(b: &[u8; REPORT_LEN]) -> Self {
        let mut args = [0; ARGS_LEN];
        args.copy_from_slice(&b[8..88]);
        Self {
            status: b[0].into(),
            tid: b[1],
            size: b[5].min(ARGS_LEN as u8),
            class: b[6],
            id: b[7],
            args,
        }
    }

    /// Whether this is the device's answer to `request`, rather than a stale
    /// response to an earlier one.
    pub fn answers(&self, request: &Report) -> bool {
        self.class == request.class && self.id == request.id
    }

    pub fn be16(&self, at: usize) -> u16 {
        u16::from_be_bytes([self.args[at], self.args[at + 1]])
    }
}

fn checksum(b: &[u8; REPORT_LEN]) -> u8 {
    b[2..88].iter().fold(0, |crc, x| crc ^ x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_the_header_and_checksum() {
        let b = Report::new(0x00, 0x81, 0x02, &[]).with_tid(0x1F).encode();
        assert_eq!(&b[..8], &[0x00, 0x1F, 0x00, 0x00, 0x00, 0x02, 0x00, 0x81]);
        assert_eq!(b[88], 0x02 ^ 0x81);
    }

    #[test]
    fn round_trips() {
        let request = Report::new(0x04, 0x85, 0x07, &[0x01, 0x06, 0x40]);
        let decoded = Report::decode(&request.encode());
        assert_eq!(decoded.args, request.args);
        assert!(decoded.answers(&request));
        assert_eq!(decoded.be16(1), 1600);
    }
}
