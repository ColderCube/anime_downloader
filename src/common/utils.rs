use regex::Regex;

// Acknowledgement https://github.com/chawyehsu/filenamify-rs/blob/main/src/lib.rs
pub fn filenamify<S: AsRef<str>>(input: S) -> String {
    let replacemant = "";
    let reserved: Regex =
        Regex::new("[<>:\"/\\\\|?*\u{0000}-\u{001F}\u{007F}\u{0080}-\u{009F}]+").unwrap();
    let windows_reserved: Regex = Regex::new("^(con|prn|aux|nul|com\\d|lpt\\d)$").unwrap();
    let outer_period: Regex = Regex::new("^\\.+|\\.+$").unwrap();

    let input = reserved.replace_all(input.as_ref(), replacemant);
    let input = outer_period.replace_all(input.as_ref(), replacemant);

    let mut result = input.into_owned();
    if windows_reserved.is_match(result.as_str()) {
        result.push_str(replacemant);
    }

    result
}

pub fn bytes_to_human_readable(bytes: u64) -> String {
    const UNITS: [&str; 3] = ["B/sec", "KB/sec", "MB/sec"];
    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    // Format with 2 decimal places
    format!("{:.2} {}", size, UNITS[unit])
}

