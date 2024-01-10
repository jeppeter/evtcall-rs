

#[cfg(target_os = "windows")]
include!("exithdl_windows.rs");

#[cfg(target_os = "linux")]
include!("exithdl_linux.rs");