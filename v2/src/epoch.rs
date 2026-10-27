//! Number(string), the original threshold, and ECMAScript TimeClip.
//! Calendar arithmetic covers the entire JS Date range without Chrono's year limit.

pub const MAX_TIME_MS: i64 = 8_640_000_000_000_000;

fn js_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
        '\u{205f}' | '\u{3000}' | '\u{feff}')
}

// Power-of-two radix strings must round once, not once per accumulated digit.
fn radix_number(text: &str, radix: u32) -> f64 {
    if text.is_empty() {
        return f64::NAN;
    }
    let width = radix.trailing_zeros();
    let mut length = 0usize;
    let mut significand = 0u64;
    let mut guard = false;
    let mut sticky = false;
    for c in text.chars() {
        let Some(digit) = c.to_digit(radix).filter(|_| c.is_ascii()) else {
            return f64::NAN;
        };
        for bit in (0..width).rev() {
            let one = (digit >> bit) & 1 != 0;
            if length == 0 && !one {
                continue;
            }
            length = length.saturating_add(1);
            match length {
                1..=53 => significand = (significand << 1) | u64::from(one),
                54 => guard = one,
                _ => sticky |= one,
            }
        }
    }
    if length > 1024 {
        return f64::INFINITY;
    }
    if guard && (sticky || significand & 1 != 0) {
        significand += 1;
    }
    (significand as f64) * 2.0_f64.powi(length.saturating_sub(53) as i32)
}

fn decimal_grammar(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i - start;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        digits += i - start;
    }
    if digits == 0 {
        return false;
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if start == i {
            return false;
        }
    }
    i == bytes.len()
}

/// JS Number(string), including empty/Unicode whitespace, radix forms and Infinity.
/// Invalid numeric strings return NaN; no parseFloat-style prefix acceptance.
pub fn number(text: &str) -> f64 {
    let text = text.trim_matches(js_whitespace);
    if text.is_empty() {
        return 0.0;
    }
    match text {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (prefixes, radix) in [(["0x", "0X"], 16), (["0b", "0B"], 2), (["0o", "0O"], 8)] {
        if let Some(digits) = prefixes.iter().find_map(|p| text.strip_prefix(p)) {
            return radix_number(digits, radix);
        }
    }
    if !decimal_grammar(text) {
        return f64::NAN;
    }
    text.parse().unwrap_or(f64::NAN)
}

/// None means a copyable "Invalid Date"; Err means V1's NaN clipboard failure.
pub fn epoch_millis(text: &str) -> Result<Option<i64>, String> {
    let epoch = number(text);
    if epoch.is_nan() {
        return Err("No number in the clipboard.".into());
    }
    // Preserve the source's exact comparison and floating-point multiplication.
    let millis = if epoch >= 2_f64.powi(31) {
        epoch
    } else {
        epoch * 1000.0
    };
    Ok((millis.is_finite() && millis.abs() <= MAX_TIME_MS as f64).then(|| millis.trunc() as i64))
}

/// Explicit offset/name makes formatting deterministic; runtime resolves the OS zone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zone {
    pub offset_seconds: i32,
    pub name: String,
}

impl Zone {
    pub fn utc() -> Self {
        Self {
            offset_seconds: 0,
            name: "Coordinated Universal Time".into(),
        }
    }
}

// Proleptic Gregorian 400-year cycles, with Euclidean division for BCE dates.
pub(crate) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let doe = shifted.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}

#[cfg(any(windows, test))]
pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// Date.toString layout, English calendar fields, full JS Date range, no milliseconds.
/// The parenthetical timezone name is supplied, as JS leaves it implementation-defined.
pub fn format_date(millis: Option<i64>, zone: &Zone) -> String {
    let Some(millis) = millis.filter(|ms| ms.unsigned_abs() <= MAX_TIME_MS as u64) else {
        return "Invalid Date".into();
    };
    let local_seconds = millis.div_euclid(1000) + i64::from(zone.offset_seconds);
    let days = local_seconds.div_euclid(86_400);
    let daytime = local_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let year = if year < 0 {
        format!("-{:04}", -year)
    } else {
        format!("{year:04}")
    };
    let weekday =
        ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][(days + 4).rem_euclid(7) as usize];
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month - 1) as usize];
    let offset_minutes = zone.offset_seconds.unsigned_abs() / 60;
    let sign = if zone.offset_seconds < 0 { '-' } else { '+' };
    let suffix = if zone.name.is_empty() {
        String::new()
    } else {
        format!(" ({})", zone.name)
    };
    format!(
        "{weekday} {month} {day:02} {year} {:02}:{:02}:{:02} GMT{sign}{:02}{:02}{suffix}",
        daytime / 3600,
        daytime / 60 % 60,
        daytime % 60,
        offset_minutes / 60,
        offset_minutes % 60
    )
}
