
trait Device
{
    fn read(&self, timeout: i32) -> u8;
    fn write(&self, data: u8, timeout: i32);
}

trait Stream
{
    fn read(&self, timeout: i32) -> u8;
    fn write(&self, data: u8, timeout: i32);
}

struct XModem
{
    uart: Box<dyn Device>,
    retries: i32,
    timeout: i32,
    padbyte: u8
}

impl XModem
{
    pub fn new(device: Box<dyn Device>) -> Self
    {
        Self {
            uart: device,
            retries: 16,
            timeout: 1000,
            padbyte: 27
        }
    }

    

}