extern crate serial;

mod xmodem;

use std::env;
use std::io;
use std::time::Duration;

use serial::prelude::*;

fn main() {
    for arg in env::args_os().skip(1) {
        let mut port = serial::open(&arg).unwrap();
        interact(&mut port).unwrap();
    }
}

fn interact<T: SerialPort>(port: &mut T) -> io::Result<()> {
    port.reconfigure(&|settings: &mut dyn SerialPortSettings| {
        settings.set_baud_rate(serial::Baud9600)?;
        settings.set_char_size(serial::Bits8);
        settings.set_parity(serial::ParityNone);
        settings.set_stop_bits(serial::Stop1);
        settings.set_flow_control(serial::FlowNone);
        Ok(())
    })?;

    port.set_timeout(Duration::from_millis(1000))?;

    let mut buf: Vec<u8> = (0..255).collect();

    port.write(&buf[..])?;
    port.read(&mut buf[..])?;

    Ok(())
}