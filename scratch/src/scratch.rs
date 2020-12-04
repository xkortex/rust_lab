use std::net::{TcpStream, UdpSocket};
use std::io::{Read, Write};
use std::str;
use chrono::{NaiveDateTime, NaiveTime, Timelike, Utc, TimeZone};
use std::time::Duration;
use std::time::SystemTime;
use std::ops::Sub;
use std::convert::TryFrom;


fn do_syscall() -> Result<(), String>{
    // let x = syscall::platform::nr::SETTIMEOFDAY;
    // let x = syscall::platform::nr::ADJTIME;
    Ok(())
}

/// $GPRMC,181804.00 - 16
fn rmc_parse(data: &[u8]) {
    let now = chrono::prelude::Utc::now();
    let date = now.date();
    let text = date.to_string() + " " + str::from_utf8(&data).unwrap();
    println!("{}", text);
    // let maybe_time = NaiveDateTime::parse_from_str(&text[..29], "%Y-%m-%d%Z $GPRMC,%H%M%S%.f");
    let maybe_time = Utc.datetime_from_str(&text[..29], "%Y-%m-%d%Z $GPRMC,%H%M%S%.f");
    match maybe_time {
        Ok(v) => {

            println!("DateTime: {}", v);
            println!("Sec/Nanos: {}.{:09}", v.second(), v.nanosecond());
            println!("Date: {} ", date.to_string());
            // date + v;
        },
        Err(e) => println!("Failed to parse time `{}`: {}", text, e),
    }
    let later = Utc::now();
    println!("now: {:?}", now);
    println!("{:?} {:?}", later - now, now - later);
    println!("{:?} {:?}", now.timestamp(), now.nanosecond());
    let s = do_syscall().unwrap();
    println!("{:?}", do_syscall());
    let s = format!("{} {}", 1, 2);
}




fn main() {
    let test = "$GPRMC,183945.00,V,,,,,,,031220,,,N*7D";
    rmc_parse(test.as_bytes())
}