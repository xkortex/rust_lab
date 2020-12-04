use std::net::{TcpStream, UdpSocket};
use std::io::{Read, Write};
use std::str;
use std::fmt;
use chrono::{NaiveDateTime, NaiveTime, Timelike, DateTime, Utc, TimeZone, Duration};
use std::time::SystemTime;
use std::ops::Sub;
use std::convert::{TryFrom, TryInto};
use nix::sys::time::TimeSpec;

/// There is probably lots of suboptimal and un-idiomatic code here. Very much learning as
/// I go along.


pub struct MyTimeSpec(nix::sys::time::TimeSpec);

pub struct MyClockId(nix::time::ClockId);


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

#[cfg(not(any(
target_os = "macos",
target_os = "ios",
all(
not(any(target_env = "uclibc", target_env = "newlibc")),
any(target_os = "redox", target_os = "hermit", ),
),
)))]
pub fn _clock_settime(clock_id: nix::time::ClockId, timespec: TimeSpec) -> Result<TimeSpec, String> {
    println!("*nix: Setting time to: {:?}", timespec);
    let ret = nix::time::clock_settime(clock_id, timespec);
    let immediately_after = nix::time::clock_gettime(clock_id);
    match ret {
        Ok(_) => {
            match immediately_after {
                Ok(t) => return Ok(t),
                Err(e) => return Err(format!("Failed to clock_gettime after clock_settime: {}", e.to_string()))
            }
        }
        Err(e) => return Err(format!("Failed to clock_settime: {}", e.to_string()))
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
pub fn _clock_settime(clock_id: nix::time::ClockId, timespec: TimeSpec) -> Result<TimeSpec, String> {
    println!("*mac: Setting time to: {:?}", timespec);
    // let ret = nix::time::clock_settime(clock_id, timespec);
    // ret.map_err(|e| e.to_string())

    Err("Unsupported platform".to_string())
}


/// probably don't need this since clock_gettime seems more widely supported
pub fn clock_gettime(clock_id: MyClockId) -> Result<MyTimeSpec, String> {
    let res = nix::time::clock_gettime(clock_id.0);
    return res.map(|ts| MyTimeSpec(ts)).map_err(|e| e.to_string());
}


/// Try to set the time, return the updated system time on success.
pub fn clock_settime(clock_id: MyClockId, timespec: MyTimeSpec) -> Result<MyTimeSpec, String> {
    let res = _clock_settime(clock_id.0, timespec.0);
    res.map(|ts| MyTimeSpec(ts))
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
fn set_time(target: DateTime<Utc>) {
    let rtc = nix::time::ClockId::CLOCK_REALTIME;
    match nix::time::clock_gettime(rtc) {
        Ok(ts) => println!("current timespec: {:?}", ts),
        Err(e) => println!("Failed to get time: {}", e)
    };
    let new_ts = MyTimeSpec::from(target);
    let latest_ts = clock_settime(MyClockId(rtc), new_ts);
    match latest_ts {
        Ok(ts) => println!("Time was set: {:?}", ts),
        Err(e) => println!("Failed to set time: {}", e),
    }
}

fn adjust_time(target: ParsedTime, now: DateTime<Utc>) -> Result<Duration, String> {
    match target.ptype {
        GpsPacketType::PPS => {
            println!("yet to do. this will eventually set the date if it's way off");
            return Ok(now - target.dt);
        }
        GpsPacketType::RMC => {
            set_time(target.dt);
            return Ok(now - target.dt);
        }
    }
    Err("Unable to set time".to_string())
}

fn udp_client() -> std::io::Result<()>  {
    let port = 12345;
    let addr = "255.255.255.255:".to_string() + &port.to_string();
    let mut socket = UdpSocket::bind(addr)?;
    println!("start UDP listener");

    let mut buf = [0 as u8; 128];
    loop {
        let (amt, src) = socket.recv_from(&mut buf)?;
        // println!("From: {:?}", src,);
        // println!("buf: {:?}", &buf[0..32]);
        let now = chrono::prelude::Utc::now();
        let res = parse_once(&buf, now);
        match res {
            Ok(pt) => {
                println!("now: {:?} | {:?} |∆ {:?}", now, pt, pt.dt - now);
                adjust_time(pt, now);
            }
            Err(e) => println!("{:?}", e),
        }
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
    udp_client()
}