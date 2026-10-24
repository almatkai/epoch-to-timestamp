//! Runtime OS timezone lookup. No fixed-offset cache; DST is resolved at the instant.
use crate::epoch::format_date;
#[cfg(not(windows))]
use crate::epoch::Zone;

pub fn format_local(millis: Option<i64>) -> Result<String, String> {
    match millis {
        None => Ok("Invalid Date".into()),
        Some(ms) => Ok(format_date(Some(ms), &local_zone(ms)?)),
    }
}

#[cfg(unix)]
pub fn local_zone(millis: i64) -> Result<Zone, String> {
    // Keep the checked conversion for Unix targets with a 32-bit time_t.
    #[allow(clippy::useless_conversion)]
    let seconds: libc::time_t = millis
        .div_euclid(1000)
        .try_into()
        .map_err(|_| "Date is outside this OS's local timezone range")?;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: valid pointers; localtime_r writes a complete tm on success. No
    // environment mutation or tzset call occurs here. TZ is owned by the host.
    let local = unsafe {
        if libc::localtime_r(&seconds, local.as_mut_ptr()).is_null() {
            return Err("OS could not resolve the date's local timezone".into());
        }
        local.assume_init()
    };
    let mut name = [0u8; 256];
    // SAFETY: NUL-terminated format, initialized tm, writable bounded buffer.
    let length =
        unsafe { libc::strftime(name.as_mut_ptr().cast(), name.len(), c"%Z".as_ptr(), &local) };
    Ok(Zone {
        offset_seconds: local
            .tm_gmtoff
            .try_into()
            .map_err(|_| "OS timezone offset overflow")?,
        name: String::from_utf8_lossy(&name[..length]).into_owned(),
    })
}

#[cfg(any(windows, test))]
#[path = "windows_rules.rs"]
mod windows_rules;

#[cfg(windows)]
#[path = "windows_local.rs"]
mod windows_local;
#[cfg(windows)]
pub use windows_local::local_zone;

#[cfg(not(any(unix, windows)))]
pub fn local_zone(_: i64) -> Result<Zone, String> {
    Err("Local timezone is unavailable on this target".into())
}
