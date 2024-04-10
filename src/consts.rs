

pub const READ_EVENT :u32 = 0x1;
pub const WRITE_EVENT :u32 = 0x2;
pub const ERROR_EVENT :u32 = 0x4;
pub const ET_TRIGGER  :u32 = 0x80;
pub const INVALID_EVENT_HANDLE :u64 = 0xffffffffffffffff;

pub const EVENT_NO_AUTO_RESET :u32 = 0x1;