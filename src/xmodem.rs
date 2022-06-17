
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

    fn send_nak(&mut self) {
        let packet: Vec<u8> = vec![NAK];
        self.uart.as_mut().write(&packet[..]).expect("Failed Send Transmission Byte");
    }

    fn send_ack(&mut self) {
        let packet: Vec<u8> = vec![ACK];
        self.uart.as_mut().write(&packet[..]).expect("Failed Send Transmission Byte");
    }

    pub fn recieve(&mut self, mut stream: Box<dyn Write>, crc_mode: bool) -> Result<usize, &'static str> {
        let mut errors = 0;
        let mut size = 0;
        let mut cancel = false;
        // Synchronization
        loop {
            if crc_mode {
                let buf = vec![CRC];
                self.uart.as_mut().write(&buf[..]).expect("Sync I/O failure");
            }
            else {
                let buf = vec![NAK];
                self.uart.as_mut().write(&buf[..]).expect("Sync I/O failure");
            }
            break;
        }
        // Receive Packets
        let mut data_length: usize = 128;
        let packet_length = if crc_mode { data_length + 4 } else { data_length + 3};
        let mut packet_num: u8 = 1;
        errors = 0;
        loop {

            let mut header = vec![0; 1];
            // Read Header
            match self.uart.as_mut().read(&mut header) {
                Ok(_) => {
                    println!("Data received {:?}", header);

                    match header[0] {
                        SOH => data_length = 128,
                        STX => data_length = 1024,
                        EOT => break,
                        CAN => 
                        {
                            if cancel {
                                return Err("Cancelled got CAN Twice")
                            }
                            cancel = true;
                        }
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
                        return Err("Packet Send Failed, reached max number of retries");
                    }
                }
            }

            // Read rest of packet.
            let mut packet = vec![0; packet_length];
            match self.uart.as_mut().read(&mut packet) {
                Ok(_) => {
                    println!("Data received {:?}", packet);

                    let pn1 = packet[0];
                    let pn2 = packet[1];
                    if (pn1 + pn2) != 0xff {
                        println!("Error Packet Number was not expected");
                        errors += 1;
                        self.send_nak();
                        continue;
                    }
                    else if pn1 != packet_num {
                        println!("Error Packet Number was not expected");
                        errors += 1;
                        self.send_nak();
                        continue;
                    }
                    if errors > self.retries {
                        return Err("Packet Send Failed, reached max number of retries");
                    }

                    if crc_mode {
                        let calc_crc = crc(&packet[2..130]);
                        let received_crc = ((packet[130] as u16) << 8) | packet[131] as u16;
                        if received_crc != calc_crc
                        {
                            println!("CRC error: theirs {received_crc}, ours {calc_crc}");
                            errors += 1;
                            self.send_nak();
                            continue;
                        }
                    }
                    else {
                        let calc_checksum = checksum(&packet[2..130]);
                        let received_checksum = packet[130];
                        if calc_checksum != received_checksum {
                            println!("Check sum error: theirs {received_checksum}, ours {calc_checksum}");
                            errors += 1;
                            self.send_nak();
                            continue;
                        }
                    }

                    size += data_length;
                    stream.as_mut().write(&packet[2..130]).expect("Failed to write to stream");
                    println!("Send ACK");
                    self.send_ack();
                    packet_num += 1;

                }
                _ => {
                    errors += 1;
                    if errors > self.retries {
                        return Err("Packet Send Failed, reached max number of retries");
                    }
                }
            }
        }
        self.send_ack();
        println!("Data received, size: {size}");
        Ok(size)
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
                            println!("Use CRC Mode");
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
                            let crc = crc(&data);
                            let hi_crc_byte: u8 = (crc >> 8) as u8;
                            let lo_crc_byte: u8 = (crc & 0xff) as u8;
                            println!("CRC: {}", crc);
                            packet.push(hi_crc_byte);
                            packet.push(lo_crc_byte);
                        }
                        else {
                            let checksum = checksum(&data);
                            println!("Checksum: {}", checksum);
                            packet.push(checksum);
                        }
                        self.uart.as_mut().write(&packet[..]).expect("Failed to Send Bytes");
                        self.uart.clear(serialport::ClearBuffer::Input).expect("Failed to clear buffer");
                        assert!(self.uart.bytes_to_read().unwrap() == 0);
                        let mut bytes = [0; 1];
                        // Get Receiver ACK
                        match self.uart.as_mut().read_exact(&mut bytes) {
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
                                        break;
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
            self.uart.clear(serialport::ClearBuffer::Input).expect("Failed to clear buffer");
            assert!(self.uart.bytes_to_read().unwrap() == 0);
            self.uart.as_mut().write(&packet[..]).expect("Failed Send Transmission Byte");
            let mut bytes = [0; 1];
            match  self.uart.as_mut().read_exact(&mut bytes) {
                Ok(_) => {
                    let byte = bytes[0];
                    println!("End Sync Received Byte: {}, Errors: {}", byte, errors);
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
                _ => return Err("End of Transmission Sync I/O failure"),
            }
        }
        Ok(())
    }





}

pub fn checksum(data: &[u8]) -> u8 {
    let sum: u32 = data.iter().map(|&val| val as u32).sum();
    let checksum = (sum % 256) as u8;
    return checksum; 
}

pub fn crc(data: &[u8]) -> u16 {
    let mut crc = 0;
    for val in data {
        let item: i32 = val.clone().into();
        crc = crc ^ (item << 8);
        for _ in 0..8 {
            crc = crc << 1;
            if crc & 0x10000 > 0 {
                crc = (crc ^ 0x1021) & 0xffff;
            }
        }
    }
    return crc as u16; 
}


#[cfg(test)]

#[test]
fn test_crc()
{
    let data: Vec<u8> = vec![0x12, 0x34, 0x56, 0x78, 0x09];
    let result = crc(&data);
    assert_eq!(result, 0x5A76);
}