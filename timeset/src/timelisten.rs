use std::net::{TcpStream, UdpSocket};
use std::io::{Read, Write};
use std::str;
use std::fmt;
use chrono::{NaiveDateTime, NaiveTime, Timelike, DateTime, Utc, TimeZone, Duration};
use std::time::SystemTime;
use std::ops::Sub;
use std::convert::{TryFrom, TryInto};
use nix::sys::time::TimeSpec;
use std::fmt::Debug;
use std::error::Error;
use crate::TimeSyncError::UnsupportedPlatform;

/// There is probably lots of suboptimal and un-idiomatic code here. Very much learning as
/// I go along.

#[derive(Clone, Debug)]
pub struct ConfigOptions {
    do_adjust: bool,
    do_once: bool,
    addr: String,
}

pub struct MyTimeSpec(nix::sys::time::TimeSpec);

#[derive(Clone, Copy)]
pub struct MyClockId(nix::time::ClockId);

/// ways in which this can fail:
/// Sys(Errno) (nix::lib::Error)
/// UnsupportedOperation (nix::lib::Error)
/// Parse Error from parsing packets
/// IO error from network stack std::io::Error

///
///
///
pub enum TimeSyncError {
    /// syscall setting time failed
    NixError(nix::Error),

    /// connection to the GPS failed or some other IOError
    IOError(std::io::Error),

    /// parsing output of GPS failed
    PacketParseFailed(chrono::ParseError),

    /// you are trying to use this on an OS which does not support the settime as implemented
    UnsupportedPlatform,

    /// this should basically never happen
    GettimeFailed(nix::Error),
}

pub type TimeSyncResult<T> = Result<T, TimeSyncError>;

impl From<std::io::Error> for TimeSyncError {
    fn from(error: std::io::Error) -> TimeSyncError {
        TimeSyncError::IOError(error)
    }
}
impl From<nix::Error> for TimeSyncError {
    fn from(error: nix::Error) -> TimeSyncError { TimeSyncError::NixError(error) }
}
impl From<chrono::ParseError> for TimeSyncError {
    fn from(error: chrono::ParseError) -> TimeSyncError { TimeSyncError::PacketParseFailed(error) }
}

fn foo() {
    let now = chrono::Utc::now();
    // let ts = now.timestamp_nanos();
    // let dur = std::time::Duration{};
    // let dur = chrono::Duration::from_std();
}

pub enum TimeSysErrors {
    /// clock_settime() does not have write permission for the dynamic POSIX clock device indicated.
    EACCES,
    /// tp points outside the accessible address space.
    EFAULT,
    /// The clockid specified is invalid for one of two reasons. Either the System-V style hard coded positive value is out of range, or the dynamic clock ID does not refer to a valid instance of a clock object. EINVAL (clock_settime()): tp.tv_sec is negative or tp.tv_nsec is outside the range [0..999,999,999]. EINVAL The clockid specified in a call to clock_settime() is not a settable clock. EINVAL (since Linux 4.3) A call to clock_settime() with a clockid of CLOCK_REALTIME attempted to set the time to a value less than the current value of the CLOCK_MONOTONIC clock.
    EINVAL,
    /// The hot-pluggable device (like USB for example) represented by a dynamic clk_id has disappeared after its character device was opened.
    ENODEV,
    /// The operation is not supported by the dynamic POSIX clock device specified.
    ENOTSUP,
    ///  clock_settime() does not have permission to set the clock indicated.
    EPERM,
}

enum TimeParseError {
    OutOfRange,
    Impossible,
    NotEnough,
    Invalid,
    TooShort,
    TooLong,
    BadFormat,
}

enum GpsPacketType {
    RMC,
    // These messages occur at the top of the second
    PPS, // These messages actually occur ~500ms before the time they describe
}

#[derive(Debug)]
pub enum TimeSetErrorKind {
    ConnectionFailed,
    TimeSetFailed,
    TimeGetFailed,
}

#[derive(Debug)]
pub struct TimeSetError {
    kind: TimeSetErrorKind,
}

impl fmt::Display for TimeSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SuperError is here!")
    }
}

impl std::error::Error for TimeSetError {}

impl std::convert::From<std::io::Error> for TimeSetError {
    fn from(e: std::io::Error) -> Self {
        return TimeSetError { kind: TimeSetErrorKind::ConnectionFailed };
    }
}


struct ParsedTime {
    dt: DateTime<Utc>,
    ptype: GpsPacketType,
}

impl fmt::Debug for GpsPacketType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GpsPacketType::RMC => write!(f, "RMC"),
            GpsPacketType::PPS => write!(f, "PPS"),
        }
    }
}

impl fmt::Debug for ParsedTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} {:?} ", self.ptype, self.dt)
    }
}

impl fmt::Debug for MyTimeSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} {:?} ", self.0.tv_sec(), self.0.tv_nsec())
    }
}

impl From<DateTime<Utc>> for MyTimeSpec {
    fn from(dt: DateTime<Utc>) -> Self {
        let secs = u64::try_from(dt.timestamp()).ok().unwrap();
        let nanos = dt.nanosecond();
        let ctd = core::time::Duration::new(secs, nanos);
        return MyTimeSpec(nix::sys::time::TimeSpec::from(ctd));
    }
}

impl MyTimeSpec {
    pub fn now() -> TimeSyncResult<MyTimeSpec> {
        clock_gettime(MyClockId(nix::time::ClockId::CLOCK_REALTIME))
    }
}

/// idk how to turn this into a trait impl
pub fn duration_to_sec(dur: Duration) -> f64 {
    let secs = dur.num_seconds() as f64;
    let nanos = match dur.num_nanoseconds() {
        Some(ns) => ns as f64,
        None => 0.0,
    };
    secs + nanos/1e9
}


#[cfg(not(any(
target_os = "macos",
target_os = "ios",
all(
not(any(target_env = "uclibc", target_env = "newlibc")),
any(target_os = "redox", target_os = "hermit", ),
),
)))]
fn _clock_settime(clock_id: nix::time::ClockId, timespec: TimeSpec) -> TimeSyncResult<TimeSpec> {
    println!("*nix: Setting time to: {:?}", timespec);
    let ret = nix::time::clock_settime(clock_id, timespec); // Result<TimeSpec>
    let immediately_after = nix::time::clock_gettime(clock_id);
    match ret {
        Ok(_) => {
            match immediately_after {
                Ok(t) => return Ok(t),
                Err(e) => return Err(TimeSyncError::GettimeFailed(e))
            }
        }
        Err(e) => return Err(TimeSyncError::NixError(e))
    }
    panic!("This should not be reachable")
}

/// silly cross-platform hack so I can compile on my mac
#[cfg(any(
target_os = "macos",
target_os = "ios",
all(
not(any(target_env = "uclibc", target_env = "newlibc")),
any(target_os = "redox", target_os = "hermit", ),
),
))]
fn _clock_settime(clock_id: nix::time::ClockId, timespec: TimeSpec) -> TimeSyncResult<TimeSpec>{
    println!("*mac: Setting time to: {:?}", timespec);
    // let ret = nix::time::clock_settime(clock_id, timespec);
    // ret.map_err(|e| e.to_string())

    Err(UnsupportedPlatform)
}


/// probably don't need this since clock_gettime seems more widely supported
pub fn clock_gettime(clock_id: MyClockId) -> TimeSyncResult<MyTimeSpec> {
    let res = nix::time::clock_gettime(clock_id.0);
    res.map(|ts| MyTimeSpec(ts)).map_err(|e| TimeSyncError::from(e))
}


/// Try to set the time, return the updated system time on success.
pub fn clock_settime(clock_id: MyClockId, timespec: MyTimeSpec) -> TimeSyncResult<MyTimeSpec> {
    let res = _clock_settime(clock_id.0, timespec.0);
    res.map(|ts| MyTimeSpec(ts)).map_err(|e| TimeSyncError::from(e))
}

fn diffnow(t: SystemTime) {
    let dur = SystemTime::now().duration_since(t);
    match dur {
        Ok(d) => println!("offset: {:?}", d),
        Err(e) => println!("oops"),
    }
}

/// This is the "friendly" entrypoint for setting system time (realtime clock)
/// There are probably more layers
pub fn set_time(target: DateTime<Utc>) -> TimeSyncResult<MyTimeSpec> {
    // let old_time = MyTimeSpec::now();
    let new_ts = MyTimeSpec::from(target);
    clock_settime(MyClockId(nix::time::ClockId::CLOCK_REALTIME), new_ts)
}

fn adjust_time(target: ParsedTime, now: DateTime<Utc>) -> Result<Duration, String> {
    match target.ptype {
        GpsPacketType::PPS => {
            println!("yet to do. this will eventually set the date if it's way off");
            Ok(now - target.dt)
        }
        GpsPacketType::RMC => {
            set_time(target.dt);
            Ok(now - target.dt)
        }
    }

}

fn udp_client(config: ConfigOptions) -> std::io::Result<()>  {
    let mut _config = config.clone();
    let mut socket = UdpSocket::bind(config.addr)?;
    println!("start UDP listener: {:?}", _config);

    let mut buf = [0 as u8; 128];
    let mut idx = 0;
    loop {
        let (amt, src) = socket.recv_from(&mut buf)?;
        // println!("From: {:?}", src,);
        // println!("buf: {:?}", &buf[0..32]);
        let now = chrono::prelude::Utc::now();
        let res = parse_once(&buf, now);
        match res {
            Ok(pt) => {

                let delta_t = if pt.dt > now {
                    pt.dt - now
                } else {
                    now - pt.dt
                };
                // let dur = Duration::nanoseconds(1);
                // dur.
                let delta_t_s = duration_to_sec(delta_t);
                println!("{} now: {:?} | {:?} |∆ {:?} {:?}", idx, now, pt, delta_t, delta_t_s);
                if _config.do_adjust {
                    adjust_time(pt, now);
                    if config.do_once { _config.do_adjust = false}
                }
            }
            Err(e) => println!("{:?}", e),
        }
        idx = 1 + idx;
    }
    println!("Terminated.");
    Ok(())
}

/// $GPRMC,183945.00,V,,,,,,,031220,,,N*7D
fn rmc_parse(data: &[u8], now: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let date = now.date();
    let text = date.to_string() + " " + str::from_utf8(&data).unwrap();
    // println!("{}", text);
    let maybe_time = Utc.datetime_from_str(&text[..29].to_string(), "%Y-%m-%d%Z $GPRMC,%H%M%S%.f");
    match maybe_time {
        Ok(v) => {
            println!("now: {} RMC DateTime: {}", now.to_string(), v);
            // println!("Sec/Nanos: {}.{:09}", v.second(), v.nanosecond());
            // println!("Date: {} ", date.to_string());
            return Ok(v)
        }
        Err(e) => println!("Failed to parse time `{}`: {}", text, e),
    }

    Err("something fell through".to_string())
}

fn pps_parse(data: &[u8], now: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let text = str::from_utf8(&data).unwrap();
    let end_idx = match text.find(" ??") {
        Some(end) => end,
        None => 0,
    };
    if end_idx > 0 {
        let maybe_time = Utc.datetime_from_str(&text[..end_idx], "%Z %y.%m.%d %H:%M:%S");
        match maybe_time {
            Ok(v) => {
                println!("now: {} PPS DateTime: {} {}.{:09}", now.to_string(), v, v.timestamp(), v.nanosecond());
                return Ok(v);
            }
            Err(e) => println!("Failed to parse time: {}", e),
        }
    } else {
        println!("Unable to parse:\n{}", text);
        return Err("Unable to parse".to_string());
    }
    Err("something fell through".to_string())
}


fn parse_once(data: &[u8], now: DateTime<Utc>) -> Result<ParsedTime, String> {
    let text = match str::from_utf8(&data.clone()) {
        Ok(t) => t,
        Err(e) => return Err(e.to_string())
    };
    // println!("packet: |{}|packet", text);


    let _start_rmc = match text.find("$GPRMC") {
        Some(_) => {
            let res = rmc_parse(&data, now);
            return res.map(|dt| ParsedTime { dt, ptype: GpsPacketType::RMC });
        }
        None => 0,
    };
    let _end_idx = match text.find(" ??") {
        Some(_) => {
            let res = pps_parse(&data, now);
            return res.map(|dt| ParsedTime { dt, ptype: GpsPacketType::PPS });
        }
        None => 0,
    };

    Err("something fell through".to_string())
}


/// this is supposed to catch all errors or something? well I can't figure it out yet.
// fn main() -> Result<(), Box<TimeSetError>>  {
fn main() -> std::io::Result<()>  {
    let listen_port = 12345;
    let addr = "255.255.255.255:".to_string() + &listen_port.to_string();

    let config = ConfigOptions{ do_adjust: true, do_once: true, addr};
    udp_client(config)
}