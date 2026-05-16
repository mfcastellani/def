use chrono::{DateTime, Datelike, Local, Timelike};

use crate::error::{DefError, DefResult};
use crate::value::Value;

pub(super) fn call_datetime_method(
    value: DateTime<Local>,
    name: &str,
    args: Vec<Value>,
) -> DefResult<Value> {
    match name {
        "format" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "datetime.format expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(format) = &args[0] else {
                return Err(DefError::Runtime(
                    "datetime.format expects a string format".to_string(),
                ));
            };

            Ok(Value::String(
                value.format(&to_chrono_format(format)).to_string(),
            ))
        }
        "day" => get_or_set_datetime_part(value, name, args, |value| i64::from(value.day())),
        "month" => get_or_set_datetime_part(value, name, args, |value| i64::from(value.month())),
        "year" => get_or_set_datetime_part(value, name, args, |value| i64::from(value.year())),
        "hour" => get_or_set_datetime_part(value, name, args, |value| i64::from(value.hour())),
        "minute" => get_or_set_datetime_part(value, name, args, |value| i64::from(value.minute())),
        "second" => get_or_set_datetime_part(value, name, args, |value| i64::from(value.second())),
        _ => Err(DefError::Runtime(format!(
            "unknown datetime method '{name}'"
        ))),
    }
}

pub(super) fn is_datetime_setter(name: &str, args_len: usize) -> bool {
    args_len == 1
        && matches!(
            name,
            "day" | "month" | "year" | "hour" | "minute" | "second"
        )
}

fn get_or_set_datetime_part(
    value: DateTime<Local>,
    name: &str,
    args: Vec<Value>,
    reader: impl Fn(DateTime<Local>) -> i64,
) -> DefResult<Value> {
    match args.as_slice() {
        [] => Ok(Value::Integer(reader(value))),
        [Value::Integer(part)] => set_datetime_part(value, name, *part).map(Value::DateTime),
        [_] => Err(DefError::Runtime(format!(
            "datetime.{name} expects an integer value when setting"
        ))),
        _ => Err(DefError::Runtime(format!(
            "datetime.{name} expects 0 or 1 argument, got {}",
            args.len()
        ))),
    }
}

fn set_datetime_part(value: DateTime<Local>, name: &str, part: i64) -> DefResult<DateTime<Local>> {
    let updated = match name {
        "day" => u32::try_from(part)
            .ok()
            .and_then(|part| value.with_day(part)),
        "month" => u32::try_from(part)
            .ok()
            .and_then(|part| value.with_month(part)),
        "year" => i32::try_from(part)
            .ok()
            .and_then(|part| value.with_year(part)),
        "hour" => u32::try_from(part)
            .ok()
            .and_then(|part| value.with_hour(part)),
        "minute" => u32::try_from(part)
            .ok()
            .and_then(|part| value.with_minute(part)),
        "second" => u32::try_from(part)
            .ok()
            .and_then(|part| value.with_second(part)),
        _ => unreachable!("expected datetime part"),
    };

    updated.ok_or_else(|| {
        DefError::Runtime(format!(
            "invalid datetime.{name} value {part} for current datetime"
        ))
    })
}

fn to_chrono_format(format: &str) -> String {
    let mut converted = String::new();
    let mut index = 0;
    let mut previous_field = DateTimeField::None;

    while index < format.len() {
        let remaining = &format[index..];

        if remaining.starts_with("yyyy") {
            converted.push_str("%Y");
            previous_field = DateTimeField::Year;
            index += 4;
        } else if remaining.starts_with("yy") {
            converted.push_str("%y");
            previous_field = DateTimeField::Year;
            index += 2;
        } else if remaining.starts_with("dd") {
            converted.push_str("%d");
            previous_field = DateTimeField::Day;
            index += 2;
        } else if remaining.starts_with("hh") {
            converted.push_str("%H");
            previous_field = DateTimeField::Hour;
            index += 2;
        } else if remaining.starts_with("ss") {
            converted.push_str("%S");
            previous_field = DateTimeField::Second;
            index += 2;
        } else if remaining.starts_with("mm") {
            if previous_field == DateTimeField::Hour {
                converted.push_str("%M");
                previous_field = DateTimeField::Minute;
            } else {
                converted.push_str("%m");
                previous_field = DateTimeField::Month;
            }
            index += 2;
        } else {
            let ch = remaining
                .chars()
                .next()
                .expect("remaining format is not empty");
            if ch == '%' {
                converted.push_str("%%");
            } else {
                converted.push(ch);
            }
            index += ch.len_utf8();
        }
    }

    converted
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DateTimeField {
    None,
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

#[cfg(test)]
mod tests {
    use super::to_chrono_format;

    #[test]
    fn translates_def_datetime_format_to_chrono_format() {
        assert_eq!(to_chrono_format("hh:mm:ss dd/mm/yyyy"), "%H:%M:%S %d/%m/%Y");
        assert_eq!(to_chrono_format("dd/mm/yy"), "%d/%m/%y");
    }
}
