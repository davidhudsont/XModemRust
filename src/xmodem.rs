
use serialport::SerialPort;
use std::{io::{Read, Write}};


pub struct XModem
{
    uart: Box<dyn SerialPort>,
    retries: i32,
    timeout: i32,
    padbyte: u8
}

const SOH: u8 = 0x01;
const STX: u8 = 0x02;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const ETB: u8 = 0x17;
const CAN: u8 = 0x18;
const SUB: u8 = 0x1A;
const CRC: u8 = 0x43;


impl XModem
{
    pub fn new(device: Box<dyn SerialPort>) -> Self
    {
        Self {
            uart: device,
            retries: 16,
            timeout: 1000,
            padbyte: 27
        }
    }

    pub fn recieve(stream: Box<dyn Write>, crc_mode: bool) -> u32 {
        todo!()
    }

    pub fn send(&mut self, mut stream: Box<dyn Read>) -> Result<(), &'static str> {
        let mut errors = 0;

        let mut cancel = false;
        let mut crc_mode = false;
        // Synchronize with Reciever
        loop {
            let mut bytes = [0; 1];
            match self.uart.as_mut().read(&mut bytes) {
                Ok(_) => {
                    let byte = bytes[0];
                    println!("Receiver Byte: {}, Errors: {}", byte, errors);
                    match byte {
                        NAK => break,
                        CRC => {
                            crc_mode = true;
                            break;
                        }
                        CAN => 
                        {
                            if cancel {
                                return Err("Cancelled got CAN Twice")
                            }
                            cancel = true;
                        }
                        EOT => return Err("Cancelled got EOT"),
                        _ => {
                            errors += 1;
                            if errors > self.retries {
                                return Err("Synchronization failed, reached max number of retries");
                            }
                        }
                    }
                }
                _ => {
                    errors += 1;
                    if errors > self.retries {
                        return Err("Synchronization failed, reached max number of retries");
                    }
                }
            }
        }

        // Send Packets
        errors = 0;
        let packet_length: usize = 128;
        let mut packet_num: u8 = 1;
        self.uart.clear(serialport::ClearBuffer::Input).expect("Failed to clear buffer");
        loop {
            let mut data: Vec<u8> = vec![0; packet_length];
            match stream.as_mut().read(&mut data) {
                Ok(0) => break,
                Ok(len) => {
                    loop {
                        // Emit Packet
                        let mut packet: Vec<u8> = vec![0];
                        let seq2: u8 = 0xff - packet_num;
                        println!("PacketNum: {}", packet_num);
                        println!("PacketNum Inverse: {}", seq2);
                        println!("Stream Data Len: {}", len);
                        println!("Data to send {:?}", data);
                        packet.push(SOH);
                        packet.push(packet_num);
                        packet.push(seq2);
                        for val in &data {
                            packet.push(val.clone());
                        }

                        if crc_mode {
                            let crc = self.crc(&data);
                            let hi_crc_byte: u8 = (crc >> 8) as u8;
                            let lo_crc_byte: u8 = (crc & 0xff) as u8;
                            println!("CRC: {}", crc);
                            packet.push(hi_crc_byte);
                            packet.push(lo_crc_byte);
                        }
                        else {
                            let checksum = self.checksum(&data);
                            println!("Checksum: {}", checksum);
                            packet.push(checksum);
                        }
                        self.uart.as_mut().write(&packet[..]).expect("Failed to Send Bytes");
                        self.uart.clear(serialport::ClearBuffer::Input).expect("Failed to clear buffer");
                        assert!(self.uart.bytes_to_read().unwrap() == 0);
                        let mut bytes = [0; 1];
                        // Get Receiver ACK
                        match self.uart.as_mut().read(&mut bytes) {
                            Ok(_) => {
                                println!("Data received {:?}", bytes);
                                let byte = bytes[0];
                                println!("Receiver Byte: {}, Errors: {}", byte, errors);
                                match byte {
                                    ACK => {
                                        if packet_num == 255 {
                                            packet_num = 0;
                                        }
                                        packet_num += 1;
                                        continue;
                                    }
                                    NAK => {
                                        errors += 1;
                                        println!("Received NAK resending");
                                        if errors > self.retries {
                                            return Err("Packet Send Failed, reached max number of retries");
                                        }
                                    }
                                    _ => {
                                        errors += 1;
                                        if errors > self.retries {
                                            return Err("Packet Send Failed, reached max number of retries");
                                        }
                                    }
                                }
                            }
                            _ => {
                                errors += 1;
                                if errors > self.retries {
                                    return Err("Packet Send Failed, reached max number of retries");
                                }
                            }
                        }
                    }
                }
                _ => {
                    return Err("IO Read Error");
                }
            } 
        }

        // End of Transmission Sync
        loop {
            let packet: Vec<u8> = vec![EOT];
            self.uart.as_mut().write(&packet[..]).expect("Failed Send Transmission Byte");
            let mut bytes = [0; 1];
            match stream.as_mut().read(&mut bytes) {
                Ok(_) => {
                    let byte = bytes[0];
                    println!("Receiver Byte: {}, Errors: {}", byte, errors);
                    match byte {
                        ACK => break,
                        _ => {
                            errors += 1;
                            if errors > self.retries {
                                return Err("End of Transmission Sync, reached max number of retries");
                            }
                        }
                    }
                }
                _ => return Err("End of Transmission Sync Failed"),
            }
        }
        Ok(())
    }

    fn checksum(&self, data: &[u8]) -> u8 {
        let sum: u32 = data.iter().map(|&val| val as u32).sum();
        println!("Sum {}", sum);
        let checksum = (sum % 256) as u8;
        println!("Checksum {}", sum);
        return checksum; 
    }

    fn crc(&self, data: &[u8]) -> u16 {
        let mut crc = 0;
        for val in data {
            let item: i32 = val.clone().into();
            crc = crc ^ (item << 8);
            for _ in 0..8 {
                crc = crc << 1;
                if crc & 0x10000 == 1 {
                    crc = (crc ^ 0x1021) & 0xffff;
                }
            }
        }
        return crc as u16; 
    }

}