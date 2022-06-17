use serialport::{DataBits, StopBits};

mod xmodem;

use xmodem::XModem;

use std::time::Duration;

use std::fs::File;

fn main() {
    let builder = serialport::new("COM11", 115200)
        .data_bits(DataBits::Eight)
        .stop_bits(StopBits::One);

    let mut port = builder.open().expect("Failed to open port");
    port.set_timeout(Duration::new(1, 0)).unwrap();

    let mut xmodem: XModem = XModem::new(port);

    // let stream = File::open("example.txt").unwrap();
    // xmodem.send(Box::new(stream)).unwrap();

    let stream = File::create("example2.txt").unwrap();
    xmodem.recieve(Box::new(stream), false).unwrap();

}

