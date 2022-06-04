extern crate serial;

use serial::prelude::*;
use std::{io::{Read, Write}, collections::btree_map};


struct XModem
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
                                return Err("Reached max number of retries");
                            }
                        }
                    }
                }
                _ => {
                    errors += 1;
                    if errors > self.retries {
                        return Err("Reached max number of retries");
                    }
                }
            }
        }

        // Send Packets
        errors = 0;
        let packet_length: usize = 128;
        let packet_num: u8 = 1;
        loop {
            let mut data: Vec<u8> = vec![0; packet_length];
            match stream.as_mut().read(&mut data) {
                Ok(0) => break,
                Ok(_) => {
                    loop {
                        // Emit Packet
                        let mut packet: Vec<u8> = vec![0];
                        let seq2: u8 = 0xff - packet_num;
                        packet.push(SOH);
                        packet.push(packet_num);
                        packet.push(seq2);
                        packet.append(&mut data);

                        if crc_mode {
                            let crc = self.crc(&data);
                            let hi_crc_byte: u8 = (crc >> 8) as u8;
                            let lo_crc_byte: u8 = (crc & 0xff) as u8;
                            packet.push(hi_crc_byte);
                            packet.push(lo_crc_byte);
                        }
                        else {
                            let checksum = self.checksum(&data);
                            packet.push(checksum);
                        }
                        self.uart.as_mut().write(&packet[..]);

                        // Get Receiver ACK
                        let mut bytes = [0; 1];
                        match self.uart.as_mut().read(&mut bytes) {
                            Ok(_) => {
                                let byte = bytes[0];
                                match byte {
                                    ACK => break,
                                    NAK => {
                                        errors += 1;
                                        if errors > self.retries {
                                            return Err("Reached max number of retries");
                                        }
                                    }
                                    _ => {
                                        errors += 1;
                                        if errors > self.retries {
                                            return Err("Reached max number of retries");
                                        }
                                    }
                                }
                            }
                            _ => {
                                errors += 1;
                                if errors > self.retries {
                                    return Err("Reached max number of retries");
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
            self.uart.as_mut().write(&packet[..]);
            let mut bytes = [0; 1];
            match stream.as_mut().read(&mut bytes) {
                Ok(_) => {
                    let byte = bytes[0];
                    match byte {
                        ACK => break,
                        _ => {
                            errors += 1;
                            if errors > self.retries {
                                return Err("Reached max number of retries");
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
        let sum: u8 = data.iter().sum();
        return ((sum as u16) % 256) as u8; 
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