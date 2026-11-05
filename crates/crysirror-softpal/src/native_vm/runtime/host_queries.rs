//! Host adapters for reviewed Game.exe extcalls 18:88 and 18:97..99.
//!
//! Values remain unshifted here: the VM applies the original MiB shift even
//! to the memory-load percentage. `None` means unavailable, not measured zero.
//! Windows uses the original APIs. Other hosts expose actual local resources;
//! Win32-only version/product and virtual-address fields remain unavailable.
//! Linux commit counters are system-wide; Apple available physical memory uses
//! free pages conservatively. Linux/Android version fields describe the kernel,
//! while Apple fields describe the product release. These are host adaptations,
//! not claims that Unix resource accounting is identical to MEMORYSTATUSEX.
//!
//! API references:
//! - https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-memorystatusex
//! - https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getversionexw
//! - https://learn.microsoft.com/en-us/windows/win32/api/sensapi/nf-sensapi-isnetworkalive
//! - https://docs.kernel.org/filesystems/proc.html
//! - https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/mach/vm_statistics.h

use std::path::Path;

pub(super) fn memory_status(selector: i32) -> Option<u64> {
    if !(0..=7).contains(&selector) {
        return None;
    }
    platform::memory_status(selector)
}

pub(super) fn disk_status(selector: i32, directory: &Path) -> Option<u64> {
    if !(0..=2).contains(&selector) {
        return None;
    }
    platform::disk_status(selector, directory)
}

/// Returns 0 for no recognized connection, 1 for LAN, or 2 for WAN.
/// This queries local interface state; it never probes an Internet endpoint.
pub(super) fn network_status() -> Option<u32> {
    platform::network_status()
}

pub(super) fn os_version(selector: i32) -> Option<u32> {
    if !(0..=8).contains(&selector) {
        return None;
    }
    platform::os_version(selector)
}

#[cfg(any(
    test,
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
))]
fn version_component(version: &str, selector: i32) -> Option<u32> {
    let index = match selector {
        0 => 0,
        1 | 5 => 1,
        2 => 2,
        _ => return None,
    };
    let component = version.split('.').nth(index)?;
    let digits: String = component.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

#[cfg(any(target_os = "linux", target_os = "android", test))]
fn proc_kib(text: &str, name: &str) -> Option<u64> {
    let line = text
        .lines()
        .find(|line| line.split(':').next() == Some(name))?;
    let mut fields = line.split_once(':')?.1.split_whitespace();
    let value: u64 = fields.next()?.parse().ok()?;
    if fields.next()? != "kB" {
        return None;
    }
    value.checked_mul(1024)
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::GetLastError,
        Storage::FileSystem::GetDiskFreeSpaceExW,
        System::{EventNotificationService::IsNetworkAlive, SystemInformation::*},
    };

    pub(super) fn memory_status(selector: i32) -> Option<u64> {
        let mut status = MEMORYSTATUSEX::default();
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        // SAFETY: the initialized buffer has the documented size and lifetime.
        if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
            return None;
        }
        Some(match selector {
            0 => status.dwMemoryLoad as u64,
            1 => status.ullTotalPhys,
            2 => status.ullAvailPhys,
            3 => status.ullTotalPageFile,
            4 => status.ullAvailPageFile,
            5 => status.ullTotalVirtual,
            6 => status.ullAvailVirtual,
            7 => status.ullAvailExtendedVirtual,
            _ => return None,
        })
    }

    pub(super) fn disk_status(selector: i32, directory: &Path) -> Option<u64> {
        let mut path: Vec<u16> = directory.as_os_str().encode_wide().collect();
        if path.contains(&0) {
            return None;
        }
        path.push(0);
        let (mut available, mut total, mut free) = (0, 0, 0);
        // SAFETY: the path is terminated and all outputs are writable u64s.
        if unsafe { GetDiskFreeSpaceExW(path.as_ptr(), &mut available, &mut total, &mut free) } == 0
        {
            return None;
        }
        Some(match selector {
            0 => available,
            1 => total,
            2 => free,
            _ => return None,
        })
    }

    pub(super) fn network_status() -> Option<u32> {
        let mut flags = 0;
        // SAFETY: IsNetworkAlive receives a writable DWORD. Last error must be
        // read immediately, even when the BOOL result is nonzero.
        let alive = unsafe { IsNetworkAlive(&mut flags) };
        if unsafe { GetLastError() } != 0 {
            return None;
        }
        Some(if alive != 0 && matches!(flags, 1 | 2) {
            flags
        } else {
            0
        })
    }

    pub(super) fn os_version(selector: i32) -> Option<u32> {
        let mut version = OSVERSIONINFOEXW::default();
        version.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;
        // SAFETY: GetVersionEx accepts the extended structure when its size is
        // provided. This intentionally retains manifest/compatibility behavior.
        if unsafe { GetVersionExW((&mut version as *mut OSVERSIONINFOEXW).cast()) } == 0 {
            return None;
        }
        Some(match selector {
            0 => version.dwMajorVersion,
            1 | 5 => version.dwMinorVersion,
            2 => version.dwBuildNumber,
            3 => version.dwPlatformId,
            4 => version.wServicePackMajor as u32,
            6 => version.wSuiteMask as u32,
            7 => version.wProductType as u32,
            8 => {
                let mut product = 0;
                // SAFETY: these fields came from GetVersionEx and output is valid.
                if unsafe {
                    GetProductInfo(
                        version.dwMajorVersion,
                        version.dwMinorVersion,
                        version.wServicePackMajor as u32,
                        version.wServicePackMinor as u32,
                        &mut product,
                    )
                } == 0
                {
                    return None;
                }
                product
            }
            _ => return None,
        })
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
))]
mod platform {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::unix::ffi::OsStrExt;

    pub(super) fn disk_status(selector: i32, directory: &Path) -> Option<u64> {
        let path = CString::new(directory.as_os_str().as_bytes()).ok()?;
        // SAFETY: statvfs fills this C structure; the CString remains alive.
        let mut status: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(path.as_ptr(), &mut status) } != 0 {
            return None;
        }
        let blocks = match selector {
            0 => status.f_bavail,
            1 => status.f_blocks,
            2 => status.f_bfree,
            _ => return None,
        };
        (blocks as u64).checked_mul(status.f_frsize as u64)
    }

    pub(super) fn network_status() -> Option<u32> {
        let mut first = std::ptr::null_mut();
        // SAFETY: getifaddrs allocates a linked list owned until freeifaddrs.
        if unsafe { libc::getifaddrs(&mut first) } != 0 {
            return None;
        }
        let mut cursor = first;
        let mut flags = 0;
        while !cursor.is_null() {
            // SAFETY: cursor is one node in the still-live getifaddrs list.
            let interface = unsafe { &*cursor };
            let active = (interface.ifa_flags & (libc::IFF_UP | libc::IFF_RUNNING) as u32)
                == (libc::IFF_UP | libc::IFF_RUNNING) as u32;
            if active
                && interface.ifa_flags & libc::IFF_LOOPBACK as u32 == 0
                && !interface.ifa_addr.is_null()
            {
                // SAFETY: the nonnull address belongs to this list node.
                let family = unsafe { (*interface.ifa_addr).sa_family as i32 };
                if family == libc::AF_INET || family == libc::AF_INET6 {
                    flags |= if interface.ifa_flags & libc::IFF_POINTOPOINT as u32 != 0 {
                        2
                    } else {
                        1
                    };
                }
            }
            cursor = interface.ifa_next;
        }
        // SAFETY: release exactly the original head returned by getifaddrs.
        unsafe { libc::freeifaddrs(first) };
        // The original wrapper rejects combined/unknown flags instead of
        // choosing a preferred connection. Point-to-point is the Unix WAN proxy.
        Some(if matches!(flags, 1 | 2) { flags } else { 0 })
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub(super) fn memory_status(selector: i32) -> Option<u64> {
        let text = std::fs::read_to_string("/proc/meminfo").ok()?;
        match selector {
            0 => {
                let total = super::proc_kib(&text, "MemTotal")?;
                let available = super::proc_kib(&text, "MemAvailable")?;
                if total == 0 {
                    return None;
                }
                Some((100u128 * total.saturating_sub(available) as u128 / total as u128) as u64)
            }
            1 => super::proc_kib(&text, "MemTotal"),
            2 => super::proc_kib(&text, "MemAvailable"),
            // These are system commit values. Linux has no identical Windows
            // process commit-budget API; do not substitute raw swap capacity.
            3 => super::proc_kib(&text, "CommitLimit"),
            4 => Some(
                super::proc_kib(&text, "CommitLimit")?
                    .saturating_sub(super::proc_kib(&text, "Committed_AS")?),
            ),
            7 => Some(0),
            _ => None,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub(super) fn memory_status(selector: i32) -> Option<u64> {
        if selector == 7 {
            return Some(0);
        }
        if !(0..=2).contains(&selector) {
            return None;
        }
        let mut total = 0u64;
        let mut size = std::mem::size_of::<u64>();
        // SAFETY: hw.memsize writes a u64, with the supplied length checked.
        if unsafe {
            libc::sysctlbyname(
                c"hw.memsize".as_ptr(),
                (&mut total as *mut u64).cast(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        } != 0
            || size != std::mem::size_of::<u64>()
        {
            return None;
        }
        if selector == 1 {
            return Some(total);
        }
        // Keep one process-lifetime send right rather than leaking a new Mach
        // host port on each script query. No task/process inspection is needed.
        static HOST: std::sync::OnceLock<libc::mach_port_t> = std::sync::OnceLock::new();
        let host = *HOST.get_or_init(|| unsafe { libc::mach_host_self() });
        let mut stats: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
        let mut count = libc::HOST_VM_INFO64_COUNT;
        // SAFETY: the count describes the correctly sized statistics buffer.
        if unsafe {
            libc::host_statistics64(
                host,
                libc::HOST_VM_INFO64,
                (&mut stats as *mut libc::vm_statistics64).cast(),
                &mut count,
            )
        } != 0
        {
            return None;
        }
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page_size <= 0 {
            return None;
        }
        // Free pages include speculative pages; do not count those twice.
        // Report this conservative available-byte measure rather than assuming
        // every inactive page is immediately reclaimable without disk writes.
        let available = (stats.free_count as u64).checked_mul(page_size as u64)?;
        if selector == 2 {
            return Some(available);
        }
        if total == 0 {
            return None;
        }
        Some((100u128 * total.saturating_sub(available) as u128 / total as u128) as u64)
    }

    pub(super) fn os_version(selector: i32) -> Option<u32> {
        if !matches!(selector, 0 | 1 | 2 | 5) {
            return None;
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            let mut bytes = [0u8; 128];
            let mut size = bytes.len();
            // SAFETY: bounded writable byte buffer; no source-level OS spoofing.
            if unsafe {
                libc::sysctlbyname(
                    c"kern.osproductversion".as_ptr(),
                    bytes.as_mut_ptr().cast(),
                    &mut size,
                    std::ptr::null_mut(),
                    0,
                )
            } != 0
                || size > bytes.len()
            {
                return None;
            }
            let version = CStr::from_bytes_until_nul(&bytes[..size])
                .ok()?
                .to_str()
                .ok()?;
            super::version_component(version, selector)
        }
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            let mut name: libc::utsname = unsafe { std::mem::zeroed() };
            // SAFETY: uname writes its documented structure; release is NUL-terminated.
            if unsafe { libc::uname(&mut name) } != 0 {
                return None;
            }
            let release = unsafe { CStr::from_ptr(name.release.as_ptr()) }
                .to_str()
                .ok()?;
            // Linux/Android expose the actual kernel release, not Windows
            // version numbers or a guessed distribution/Android release.
            super::version_component(release, selector)
        }
    }
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
)))]
mod platform {
    use super::*;
    pub(super) fn memory_status(_: i32) -> Option<u64> {
        None
    }
    pub(super) fn disk_status(_: i32, _: &Path) -> Option<u64> {
        None
    }
    pub(super) fn network_status() -> Option<u32> {
        None
    }
    pub(super) fn os_version(_: i32) -> Option<u32> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selectors_preserve_the_native_minor_version_alias() {
        assert_eq!(version_component("6.8.12-android", 0), Some(6));
        assert_eq!(version_component("6.8.12-android", 2), Some(12));
        assert_eq!(version_component("26.2.1", 5), Some(2));
        assert_eq!(version_component("26.2.1", 3), None);
        assert_eq!(version_component("unknown", 0), None);
        assert_eq!(memory_status(-1), None);
        assert_eq!(memory_status(8), None);
        assert_eq!(disk_status(3, Path::new(".")), None);
        assert_eq!(os_version(9), None);
    }
    #[test]
    fn proc_values_are_bytes_and_missing_values_are_not_zero() {
        let text = "MemTotal: 8589934592 kB\nMemAvailable: 1024 kB\nNoUnit: 99\n";
        assert_eq!(proc_kib(text, "MemTotal"), Some(8_796_093_022_208));
        assert_eq!(proc_kib(text, "MemAvailable"), Some(1_048_576));
        assert_eq!(proc_kib(text, "Unknown"), None);
        assert_eq!(proc_kib(text, "NoUnit"), None);
        assert_eq!(proc_kib("X: 18446744073709551615 kB", "X"), None);
    }
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    #[test]
    fn host_queries_report_real_memory_and_volume_capacity() {
        assert!(memory_status(1).expect("physical memory query") > 0);
        assert!(disk_status(1, Path::new(".")).expect("volume query") > 0);
        assert!(os_version(0).expect("host version query") > 0);
        assert!(network_status().is_some_and(|status| status <= 2));
    }
}
