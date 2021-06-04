use std::io;
use std::str;
use std::ops::Sub;
use std::convert::TryFrom;
use std::io::Read;



fn main() {
    let mut reader = io::stdin();
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer);

    eprintln!("{:?}", buffer);
    let mut lines = buffer.split("\n").collect::<Vec<_>>();


    let len: u32 =        lines[0].parse().expect("Not an integer!");

    eprintln!("{:?}", lines);
    eprintln!("{:?}", len);

    let pairs: Vec<&str> = lines[1..].map(|el| el.split(", "));

}
        // .map(|x| x.parse().expect("Not an integer!"))
        // .collect();
