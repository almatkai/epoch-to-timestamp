use crate::epoch::{civil_from_days, Zone};
use crate::local::windows_rules::{is_daylight, Rule};
use windows_sys::Win32::{
    Foundation::SYSTEMTIME,
    System::Time::{
        GetDynamicTimeZoneInformation, GetTimeZoneInformationForYear,
        DYNAMIC_TIME_ZONE_INFORMATION, TIME_ZONE_INFORMATION,
    },
};

fn recurring(st: SYSTEMTIME) -> Result<Rule, String> {
    if st.wYear != 0 {
        return Err("Windows returned a non-recurring timezone transition".into());
    }
    Ok(Rule {
        month: st.wMonth.into(),
        weekday: st.wDayOfWeek.into(),
        occurrence: st.wDay.into(),
        second: i64::from(st.wHour) * 3600 + i64::from(st.wMinute) * 60 + i64::from(st.wSecond),
    })
}

fn name(text: &[u16]) -> String {
    String::from_utf16_lossy(&text[..text.iter().position(|v| *v == 0).unwrap_or(text.len())])
}

fn offset(bias: i32, adjustment: i32) -> Result<i32, String> {
    bias.checked_add(adjustment)
        .and_then(|v| v.checked_mul(-60))
        .ok_or_else(|| "Windows timezone offset overflow".into())
}

pub fn local_zone(millis: i64) -> Result<Zone, String> {
    let seconds = millis.div_euclid(1000);
    let (year, _, _) = civil_from_days(seconds.div_euclid(86400));
    let mut dynamic = std::mem::MaybeUninit::<DYNAMIC_TIME_ZONE_INFORMATION>::uninit();
    let mut info = std::mem::MaybeUninit::<TIME_ZONE_INFORMATION>::uninit();
    // SAFETY: OS writes complete structures on success. 0xffffffff is failure;
    // the current DST return code is not used to determine DST at the target date.
    let (dynamic, info) = unsafe {
        if GetDynamicTimeZoneInformation(dynamic.as_mut_ptr()) == u32::MAX {
            return Err("Windows could not read the local timezone".into());
        }
        let dynamic = dynamic.assume_init();
        // Windows' historical-rule API covers years 1601..30827. Like Chrono's
        // Windows backend, use endpoint rules outside that interval, then evaluate
        // recurring transition dates in the ORIGINAL year, without clipping Date.
        if GetTimeZoneInformationForYear(
            year.clamp(1601, 30827) as u16,
            &dynamic,
            info.as_mut_ptr(),
        ) == 0
        {
            return Err("Windows could not resolve timezone rules for this date".into());
        }
        (dynamic, info.assume_init())
    };
    let std_offset = offset(info.Bias, info.StandardBias)?;
    let daylight = dynamic.DynamicDaylightTimeDisabled == 0
        && info.DaylightDate.wMonth != 0
        && info.StandardDate.wMonth != 0
        && is_daylight(
            seconds,
            year,
            std_offset,
            offset(info.Bias, info.DaylightBias)?,
            recurring(info.DaylightDate)?,
            recurring(info.StandardDate)?,
        )?;
    Ok(if daylight {
        Zone {
            offset_seconds: offset(info.Bias, info.DaylightBias)?,
            name: name(&info.DaylightName),
        }
    } else {
        Zone {
            offset_seconds: std_offset,
            name: name(&info.StandardName),
        }
    })
}
