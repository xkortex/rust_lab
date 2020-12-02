use std::net::{TcpStream};
use std::io::{Read, Write};
use std::str;
use chrono::{NaiveDateTime, Timelike};
use std::time::Duration;
use std::time::SystemTime;
use std::ops::Sub;
use std::convert::TryFrom;

fn ts_to_systime(ts: nix::sys::time::TimeSpec) -> SystemTime {
    let secs = u64::try_from(ts.tv_sec()).ok().unwrap();
    let nanos = u32::try_from(ts.tv_nsec()).ok().unwrap();
    let d = Duration::new(secs, nanos);
    return std::time::UNIX_EPOCH + d
}

fn now() -> SystemTime {
    let rtc = nix::time::ClockId::CLOCK_REALTIME;
    match nix::time::clock_gettime(rtc) {
        Ok(ts) => {
            println!("Retrieved time: {}", ts);
            return ts_to_systime(ts)
        },
        Err(e) => panic!("Failed to get time: {}", e)
    };
}

fn diffnow(t: SystemTime) {
 let dur = SystemTime::now().duration_since(t);
    match dur {
        Ok(d) => println!("offset: {:?}", d),
        Err(e) => println!("oops"),
    }
}

fn set_time(time: NaiveDateTime) {
    let rtc = nix::time::ClockId::CLOCK_REALTIME;
    match nix::time::clock_gettime(rtc) {
        Ok(ts) => println!("timespec: {}", ts),
        Err(e) => println!("Failed to get time: {}", e)
    };
}

fn client() {

    match TcpStream::connect("192.168.88.99:5018") {
        Ok(mut stream) => {
            let _time = parse_once(&mut stream);
            match _time {
                Ok(time) => {
                    now();
                }
                Err(e) => println!("{}", e)
            }
        }
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
    println!("Terminated.");
}


fn parse_once(stream: &mut TcpStream) -> Result<NaiveDateTime, String> {

            let msg = b"Hello!";

            stream.write(msg).unwrap();
            println!("Sent Hello, awaiting reply...");

            let mut data = [0 as u8; 50]; // using 50 byte buffer
            match stream.read(&mut data) {
                Ok(_) => {
                    let text = str::from_utf8(&data).unwrap();
                    let end_idx = match text.find(" ??") {
                        Some(end) => end,
                        None => 0,

                    };
                    if end_idx > 0 {
                        let maybe_time = NaiveDateTime::parse_from_str(&text[..end_idx], "%Z %y.%m.%d %H:%M:%S");
                        match maybe_time {
                            Ok(v) => {
                                println!("Time: {} {}.{:09}", v, v.timestamp(), v.nanosecond());
                                return Ok(maybe_time.unwrap())
                            },
                            Err(e) => println!("Failed to parse time: {}", e),
                        }
                    } else {
                        println!("Unable to parse:\n{}", text);
                        return Err("Unable to parse".to_string())
                    }
                }
                Err(e) => {
                    println!("Failed to receive data: {}", e);
                    return Err("Failed to receive data".to_string());
                }
            }
    Err("something fell through".to_string())

}

fn main() {
    client();
}