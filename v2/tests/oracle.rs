use serde_json::Value;
use tezbar_epoch_to_timestamp_v2::epoch::{epoch_millis, format_date, number, Zone, MAX_TIME_MS};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/v1-vectors.json")).unwrap()
}

#[test]
fn untouched_v1_number_threshold_timeclip_and_utc_oracle() {
    let fixtures = fixture();
    for v in fixtures["utc"].as_array().unwrap() {
        let input = v["input"].as_str().unwrap();
        if let Some(error) = v["error"].as_str() {
            assert!(number(input).is_nan(), "{input:?}");
            assert_eq!(epoch_millis(input).unwrap_err(), error, "{input:?}");
        } else {
            assert_eq!(
                format!("{:016x}", number(input).to_bits()),
                v["bits"].as_str().unwrap(),
                "Number({input:?})"
            );
            let millis = epoch_millis(input).unwrap();
            assert_eq!(millis, v["milliseconds"].as_i64(), "TimeClip({input:?})");
            assert_eq!(
                format_date(millis, &Zone::utc()),
                v["text"].as_str().unwrap(),
                "Date({input:?})"
            );
        }
    }
}

#[test]
fn local_numeric_layout_with_explicit_oracle_offsets() {
    for zone in fixture()["zones"].as_array().unwrap() {
        for v in zone["vectors"].as_array().unwrap() {
            let result = format_date(
                v["milliseconds"].as_i64(),
                &Zone {
                    offset_seconds: v["offsetSeconds"].as_i64().unwrap() as i32,
                    name: String::new(),
                },
            );
            assert_eq!(
                result,
                v["numericText"].as_str().unwrap(),
                "{} {}",
                zone["zone"],
                v["input"]
            );
        }
    }
}

#[test]
fn entire_js_date_range_and_invalid_extremes_never_panic() {
    assert_eq!(
        format_date(Some(MAX_TIME_MS), &Zone::utc()),
        "Sat Sep 13 275760 00:00:00 GMT+0000 (Coordinated Universal Time)"
    );
    assert_eq!(
        format_date(Some(-MAX_TIME_MS), &Zone::utc()),
        "Tue Apr 20 -271821 00:00:00 GMT+0000 (Coordinated Universal Time)"
    );
    for ms in [i64::MIN, i64::MAX, MAX_TIME_MS + 1, -MAX_TIME_MS - 1] {
        assert_eq!(format_date(Some(ms), &Zone::utc()), "Invalid Date");
    }
}

// Each child starts with its own TZ environment; never mutate a test process's TZ
// while other threads may call localtime. The package runs without Node installed.
#[cfg(unix)]
#[test]
fn actual_local_timezone_helper_matches_oracle_wall_time_and_gmt_offset() {
    const CHILD: &str = "TEZBAR_EPOCH_ZONE_CHILD";
    if let Ok(zone) = std::env::var(CHILD) {
        let fixture = fixture();
        let vectors = fixture["zones"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["zone"] == zone)
            .unwrap();
        for v in vectors["vectors"].as_array().unwrap() {
            let ms = v["milliseconds"].as_i64().unwrap();
            let resolved = tezbar_epoch_to_timestamp_v2::local::local_zone(ms).unwrap();
            assert_eq!(
                i64::from(resolved.offset_seconds),
                v["offsetSeconds"].as_i64().unwrap(),
                "{} {}",
                zone,
                v["input"]
            );
            let text = tezbar_epoch_to_timestamp_v2::local::format_local(Some(ms)).unwrap();
            assert_eq!(
                text.split(" (").next().unwrap(),
                v["numericText"].as_str().unwrap(),
                "{} {}",
                zone,
                v["input"]
            );
        }
        if zone == "UTC" {
            for ms in [-MAX_TIME_MS, MAX_TIME_MS] {
                let resolved = tezbar_epoch_to_timestamp_v2::local::local_zone(ms).unwrap();
                assert_eq!(resolved.offset_seconds, 0);
                assert_eq!(
                    format_date(Some(ms), &resolved).split(" (").next(),
                    format_date(Some(ms), &Zone::utc()).split(" (").next()
                );
            }
        }
        return;
    }
    for zone in fixture()["zones"].as_array().unwrap() {
        let zone = zone["zone"].as_str().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "actual_local_timezone_helper_matches_oracle_wall_time_and_gmt_offset",
                "--nocapture",
            ])
            .env("TZ", zone)
            .env(CHILD, zone)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{zone}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
