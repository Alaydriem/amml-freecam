use crate::structs::ToastKind;
use crate::structs::ui::ui_snapshot::TOAST_FILE_CAPACITY;

// how long a screenshot toast stays
pub const TOAST_SECONDS: f64 = 3.0;

// the bar's screenshot toast: its kind, the file name it names and the seconds it stays
pub struct Toast {
    kind: ToastKind,
    // seconds the toast stays, 0 when there is none
    left: f64,
    file: [u8; TOAST_FILE_CAPACITY],
    file_len: u8,
}

impl Toast {
    pub fn new() -> Self {
        Self { kind: ToastKind::None, left: 0.0, file: [0; TOAST_FILE_CAPACITY], file_len: 0 }
    }

    pub fn kind(&self) -> ToastKind {
        self.kind
    }

    // the UTF-8 file name buffer; the first `file_len` bytes are used
    pub fn file(&self) -> [u8; TOAST_FILE_CAPACITY] {
        self.file
    }

    pub fn file_len(&self) -> u8 {
        self.file_len
    }

    // copies at most TOAST_FILE_CAPACITY - 1 bytes of the name, cut on a character boundary
    pub fn show(&mut self, kind: ToastKind, file_name: &str) {
        let mut len = file_name.len().min(TOAST_FILE_CAPACITY - 1);
        while !file_name.is_char_boundary(len) {
            len -= 1;
        }
        self.file[..len].copy_from_slice(&file_name.as_bytes()[..len]);
        self.file_len = len as u8;
        self.kind = kind;
        self.left = TOAST_SECONDS;
    }

    // counts the toast down by `dt` seconds and drops it when its time is over
    pub fn tick(&mut self, dt: f64) {
        if self.left <= 0.0 {
            return;
        }
        self.left = (self.left - dt).max(0.0);
        if self.left == 0.0 {
            self.kind = ToastKind::None;
            self.file_len = 0;
        }
    }
}

impl Default for Toast {
    fn default() -> Self {
        Self::new()
    }
}
