use std::time::{SystemTime, UNIX_EPOCH};

/// Relogio abstrato: producao usa o sistema, testes injetam `FixedClock`.
pub trait Clock {
    /// Milissegundos desde a epoch Unix.
    fn now_millis(&self) -> u64;
    /// Carimbo ISO-8601 com milissegundos e sufixo `Z`.
    fn now_iso(&self) -> String {
        format_iso_millis(self.now_millis())
    }
}

/// Relogio do sistema.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Formato ISO-8601 com milissegundos e sufixo Z.
/// Espelha `datetime.now(UTC).isoformat(timespec="milliseconds")` do Python.
pub fn format_iso_millis(millis: u64) -> String {
    let secs = millis / 1000;
    let ms = millis % 1000;
    let (y, mo, d, h, mi, s) = civil_from_secs(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ms:03}Z")
}

fn civil_from_secs(secs: u64) -> (i64, u64, u64, u64, u64, u64) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, mo, d) = civil_from_days(days);
    (y, mo, d, rem / 3600, (rem % 3600) / 60, rem % 60)
}

// Algoritmo civil-from-days de Howard Hinnant (epoch 1970-01-01).
fn civil_from_days(z: i64) -> (i64, u64, u64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u64;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u64;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Dia da semana em ingles curto (RFC 2822 usa ingles; `C` locale do shell).
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
/// Meses em ingles curto.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Data `YYYY-MM-DD` (para `Last-Update:`, espelha `date +%Y-%m-%d`).
pub fn format_ymd(millis: u64) -> String {
    let (y, mo, d, _, _, _) = civil_from_secs(millis / 1000);
    format!("{y:04}-{mo:02}-{d:02}")
}

/// Data RFC 2822 em UTC (para `Date:`, espelha `LC_ALL=C date -R`).
pub fn format_rfc2822(millis: u64) -> String {
    let secs = millis / 1000;
    let days = (secs / 86_400) as i64;
    // 1970-01-01 foi quinta-feira (indice 4 com domingo em 0).
    let weekday = WEEKDAYS[((days + 4).rem_euclid(7)) as usize];
    let (y, mo, d, h, mi, s) = civil_from_secs(secs);
    let month = MONTHS[(mo - 1) as usize];
    format!("{weekday}, {d:02} {month} {y:04} {h:02}:{mi:02}:{s:02} +0000")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_formats_with_millis_and_z() {
        assert_eq!(format_iso_millis(0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn known_timestamp_rounds_down_to_millis() {
        // 2026-10-26T00:00:00Z
        assert_eq!(
            format_iso_millis(1_792_972_800_000),
            "2026-10-26T00:00:00.000Z"
        );
        assert_eq!(
            format_iso_millis(1_792_972_800_007),
            "2026-10-26T00:00:00.007Z"
        );
    }

    #[test]
    fn formats_ymd_and_rfc2822() {
        assert_eq!(format_ymd(0), "1970-01-01");
        assert_eq!(format_ymd(1_792_972_800_000), "2026-10-26");
        assert_eq!(format_rfc2822(0), "Thu, 01 Jan 1970 00:00:00 +0000");
        assert_eq!(
            format_rfc2822(1_792_972_800_000),
            "Mon, 26 Oct 2026 00:00:00 +0000"
        );
    }
}
