//! Process/system memory helpers for adaptive performance controls.

#![allow(dead_code)]

#[cfg(target_os = "macos")]
pub fn physical_ram_bytes() -> Option<u64> {
    use std::ffi::CString;
    use std::mem::size_of;
    use std::ptr;

    let key = CString::new("hw.memsize").ok()?;
    let mut value: u64 = 0;
    let mut len = size_of::<u64>();
    let rc = unsafe {
        libc::sysctlbyname(
            key.as_ptr(),
            &mut value as *mut u64 as *mut libc::c_void,
            &mut len,
            ptr::null_mut(),
            0,
        )
    };
    if rc == 0 && len == size_of::<u64>() {
        Some(value)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
pub fn physical_ram_bytes() -> Option<u64> {
    let mut info = std::mem::MaybeUninit::<libc::sysinfo>::uninit();
    let rc = unsafe { libc::sysinfo(info.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    let info = unsafe { info.assume_init() };
    Some((info.totalram as u64).saturating_mul(info.mem_unit as u64))
}

#[cfg(target_os = "windows")]
pub fn physical_ram_bytes() -> Option<u64> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn physical_ram_bytes() -> Option<u64> {
    None
}

#[cfg(target_os = "macos")]
pub fn current_rss_bytes() -> Option<u64> {
    let mut info = libc::mach_task_basic_info {
        virtual_size: 0,
        resident_size: 0,
        resident_size_max: 0,
        user_time: libc::time_value_t { seconds: 0, microseconds: 0 },
        system_time: libc::time_value_t { seconds: 0, microseconds: 0 },
        policy: 0,
        suspend_count: 0,
    };

    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    #[allow(deprecated)]
    let kr = unsafe {
        libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            (&mut info as *mut libc::mach_task_basic_info).cast(),
            &mut count,
        )
    };
    if kr == libc::KERN_SUCCESS {
        Some(info.resident_size as u64)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
pub fn current_rss_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let mut fields = statm.split_whitespace();
    let _size_pages = fields.next()?;
    let rss_pages = fields.next()?.parse::<u64>().ok()?;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size <= 0 {
        return None;
    }
    Some(rss_pages.saturating_mul(page_size as u64))
}

#[cfg(target_os = "windows")]
pub fn current_rss_bytes() -> Option<u64> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn current_rss_bytes() -> Option<u64> {
    None
}
