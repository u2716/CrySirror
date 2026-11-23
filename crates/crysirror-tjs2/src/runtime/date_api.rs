//! Native Date calendar semantics (krkrz fd5c4baa, tjs2/tjsDate.cpp).
//! Date uses wall-clock seconds/local calendar time, independently of the
//! simulation clock used by timers, animation and RandomGenerator.
use super::value::{ObjectHandle, Variant};
use super::{Runtime, TjsHost};
use crate::error::{Result, TjsError};
use chrono::{Datelike, Duration, FixedOffset, Local, NaiveDate, TimeZone, Timelike, Utc};

fn error() -> TjsError {
    TjsError::runtime("invalid Date timestamp")
}
fn integer(args: &[Variant], i: usize, default: i64) -> Result<i64> {
    match args.get(i) {
        None | Some(Variant::Void) => Ok(default),
        Some(v) => v.to_integer(),
    }
}
fn local(seconds: i64) -> Result<chrono::DateTime<Local>> {
    Utc.timestamp_opt(seconds, 0)
        .single()
        .map(|d| d.with_timezone(&Local))
        .ok_or_else(error)
}
fn fields(y: i64, m: i64, d: i64, h: i64, min: i64, sec: i64) -> Result<chrono::NaiveDateTime> {
    let year = y.checked_add(m.div_euclid(12)).ok_or_else(error)?;
    let base = NaiveDate::from_ymd_opt(
        i32::try_from(year).map_err(|_| error())?,
        m.rem_euclid(12) as u32 + 1,
        1,
    )
    .and_then(|d| d.and_hms_opt(0, 0, 0))
    .ok_or_else(error)?;
    let seconds = d
        .checked_sub(1)
        .and_then(|v| v.checked_mul(86400))
        .and_then(|v| h.checked_mul(3600).and_then(|n| v.checked_add(n)))
        .and_then(|v| min.checked_mul(60).and_then(|n| v.checked_add(n)))
        .and_then(|v| v.checked_add(sec))
        .ok_or_else(error)?;
    base.checked_add_signed(Duration::try_seconds(seconds).ok_or_else(error)?)
        .ok_or_else(error)
}
/// C's constructor zeroes tm_isdst. Determine the local standard offset rather
/// than treating every explicit calendar constructor as current summer time.
fn standard_offset(year: i32) -> Result<i32> {
    [1, 7]
        .into_iter()
        .filter_map(|m| Local.with_ymd_and_hms(year, m, 1, 12, 0, 0).earliest())
        .map(|d| d.offset().local_minus_utc())
        .min()
        .ok_or_else(error)
}
fn from_fields(values: [i64; 6], offset: i32) -> Result<i64> {
    let naive = fields(
        values[0], values[1], values[2], values[3], values[4], values[5],
    )?;
    FixedOffset::east_opt(offset)
        .and_then(|z| z.from_local_datetime(&naive).single())
        .map(|d| d.timestamp())
        .ok_or_else(error)
}
fn parsed(s: &str) -> Result<i64> {
    let f = crate::frontend::date::parse_date_fields(s)?;
    let naive = fields(
        f.year.into(),
        i64::from(f.month) - 1,
        f.month_day.into(),
        f.hour.into(),
        f.minute.into(),
        f.second.into(),
    )?;
    if f.timezone_seconds.is_some() || f.timezone_offset_seconds.is_some() {
        let offset = f
            .timezone_seconds
            .unwrap_or(0)
            .checked_add(f.timezone_offset_seconds.unwrap_or(0))
            .ok_or_else(error)?;
        FixedOffset::east_opt(offset)
            .and_then(|z| z.from_local_datetime(&naive).single())
            .map(|d| d.timestamp())
            .ok_or_else(error)
    } else {
        Local
            .from_local_datetime(&naive)
            .earliest()
            .map(|d| d.timestamp())
            .ok_or_else(error)
    }
}
fn object<H: TjsHost + 'static>(
    rt: &Runtime<H>,
    this: Option<ObjectHandle>,
) -> Result<ObjectHandle> {
    let h = this
        .map(|h| rt.bound_this(h).unwrap_or(h))
        .ok_or_else(error)?;
    if !rt.has_object_member(h, "__date_seconds") {
        return Err(error());
    }
    Ok(h)
}
fn seconds<H: TjsHost + 'static>(rt: &Runtime<H>, h: ObjectHandle) -> Result<i64> {
    rt.object_member(h, "__date_seconds").to_integer()
}
pub(super) fn install_class<H: TjsHost + 'static>(rt: &mut Runtime<H>, class: ObjectHandle) {
    rt.register_object_native(class, "getTimezoneOffset", |_: &mut Runtime<H>, _, _| {
        Ok(Variant::Integer(
            -i64::from(standard_offset(Local::now().year())?) / 60,
        ))
    });
}
pub(super) fn construct<H: TjsHost + 'static>(
    rt: &mut Runtime<H>,
    args: Vec<Variant>,
) -> Result<Variant> {
    let stamp = match args.first() {
        None => Utc::now().timestamp(),
        Some(Variant::String(s)) => parsed(s)?,
        Some(_) => {
            let y = integer(&args, 0, 0)?;
            let v = [
                y,
                integer(&args, 1, 0)?,
                integer(&args, 2, 1)?,
                integer(&args, 3, 0)?,
                integer(&args, 4, 0)?,
                integer(&args, 5, 0)?,
            ];
            from_fields(v, standard_offset(i32::try_from(y).map_err(|_| error())?)?)?
        }
    };
    let h = rt.alloc_ordinary_object();
    rt.add_object_class_info(h, "Date");
    rt.set_object_member(h, "__date_seconds", Variant::Integer(stamp));
    for (name, field) in [
        ("getYear", 0),
        ("getMonth", 1),
        ("getDate", 2),
        ("getHours", 3),
        ("getMinutes", 4),
        ("getSeconds", 5),
        ("getDay", 6),
        ("getTime", 7),
        ("getTimezoneOffset", 8),
    ] {
        rt.register_object_native(h, name, move |rt: &mut Runtime<H>, this, _| {
            let h = object(rt, this)?;
            let stamp = seconds(rt, h)?;
            if field == 7 {
                return Ok(Variant::Integer(stamp.checked_mul(1000).ok_or_else(error)?));
            }
            let d = local(stamp)?;
            let v = match field {
                0 => i64::from(d.year()),
                1 => i64::from(d.month0()),
                2 => i64::from(d.day()),
                3 => i64::from(d.hour()),
                4 => i64::from(d.minute()),
                5 => i64::from(d.second()),
                6 => i64::from(d.weekday().num_days_from_sunday()),
                _ => -i64::from(standard_offset(d.year())?) / 60,
            };
            Ok(Variant::Integer(v))
        });
    }
    for (name, field) in [
        ("setYear", 0),
        ("setMonth", 1),
        ("setDate", 2),
        ("setHours", 3),
        ("setMinutes", 4),
        ("setSeconds", 5),
        ("setTime", 6),
        ("parse", 7),
    ] {
        rt.register_object_native(
            h,
            name,
            move |rt: &mut Runtime<H>, this, args: Vec<Variant>| {
                if args.is_empty() {
                    return Err(TjsError::bad_param_count());
                }
                let h = object(rt, this)?;
                let stamp = if field == 7 {
                    parsed(&args[0].to_tjs_string()?)?
                } else if field == 6 {
                    args[0].to_integer()? / 1000
                } else {
                    let d = local(seconds(rt, h)?)?;
                    let mut v = [
                        i64::from(d.year()),
                        i64::from(d.month0()),
                        i64::from(d.day()),
                        i64::from(d.hour()),
                        i64::from(d.minute()),
                        i64::from(d.second()),
                    ];
                    v[field] = args[0].to_integer()?;
                    // Native setters preserve localtime's tm_isdst choice.
                    from_fields(v, d.offset().local_minus_utc())?
                };
                rt.set_object_member(h, "__date_seconds", Variant::Integer(stamp));
                Ok(Variant::Void)
            },
        );
    }
    super::builtins::install_empty_finalize(rt, h);
    Ok(Variant::self_bound(h))
}
