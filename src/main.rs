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
    // port.reconfigure(&|settings: &mut dyn SerialPortSettings| {
    //     settings.set_baud_rate(serial::Baud115200)?;
    //     settings.set_char_size(serial::Bits8);
    //     settings.set_parity(serial::ParityNone);
    //     settings.set_stop_bits(serial::Stop1);
    //     settings.set_flow_control(serial::FlowNone);
    //     Ok(())
    // }).unwrap();
    let mut xmodem: XModem = XModem::new(port);

    let stream = File::open("example.txt").unwrap();

    xmodem.send(Box::new(stream)).unwrap();

    // interact(&mut port).unwrap();
}

// fn interact<T: SerialPort>(port: &mut T) -> io::Result<()> {
//     port.reconfigure(&|settings: &mut dyn SerialPortSettings| {
//         settings.set_baud_rate(serial::Baud9600)?;
//         settings.set_char_size(serial::Bits8);
//         settings.set_parity(serial::ParityNone);
//         settings.set_stop_bits(serial::Stop1);
//         settings.set_flow_control(serial::FlowNone);
//         Ok(())
//     })?;

//     port.set_timeout(Duration::from_millis(1000))?;

//     let mut buf: Vec<u8> = (0..255).collect();

//     port.write(&buf[..])?;
//     port.read(&mut buf[..])?;

//     Ok(())
// }