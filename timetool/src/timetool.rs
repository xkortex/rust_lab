use sntpc;

fn main() {
    let result = sntpc::request("pool.ntp.org", 123);
    if let Ok(sntpc::NtpResult {
                  sec, nsec, roundtrip, offset
              }) = result {
        println!("NTP server time: {}.{}", sec, nsec);
        println!("Roundtrip time: {}, offset: {}", roundtrip, offset);
    }
}