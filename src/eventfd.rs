
#[cfg(target_os = "windows")]
include!("eventfd_windows.rs");

#[cfg(target_os = "linux")]
include!("eventfd_linux.rs");

