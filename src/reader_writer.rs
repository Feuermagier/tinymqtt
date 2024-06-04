use super::flags::Flags;


pub struct MqttMessageWriter<'a> {
    buffer: &'a mut [u8],
    cursor: usize,
}

impl<'a> MqttMessageWriter<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self { buffer, cursor: 0 }
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buffer[self.cursor] = value;
        self.cursor += 1;
    }

    pub fn write_flags(&mut self, value: Flags) {
        self.write_u8(value.value);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.buffer[self.cursor] = (value >> 8) as u8;
        self.buffer[self.cursor + 1] = value as u8;
        self.cursor += 2;
    }

    pub fn write_string(&mut self, value: &str) {
        self.write_u16(value.len() as u16);
        for byte in value.bytes() {
            self.buffer[self.cursor] = byte;
            self.cursor += 1;
        }
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.write_u16(value.len() as u16);
        self.write_bytes_raw(value);
    }

    pub fn write_bytes_raw(&mut self, value: &[u8]) {
        for byte in value {
            self.buffer[self.cursor] = *byte;
            self.cursor += 1;
        }
    }

    pub fn write_variable_int(&mut self, mut value: u32) {
        loop {
            // Successively take the last 7 bits, check if anything remaining (i.e. > 0),
            // and repeat
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value > 0 {
                byte |= 0x80;
            }
            self.buffer[self.cursor] = byte;
            self.cursor += 1;
            if value == 0 {
                break;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.cursor
    }
}

pub struct MqttMessageReader<'a> {
    buffer: &'a [u8],
    cursor: usize,
    mark: usize,
}

impl<'a> MqttMessageReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, cursor: 0, mark: 0 }
    }

    pub fn read_u8(&mut self) -> u8 {
        let value = self.buffer[self.cursor];
        self.cursor += 1;
        value
    }

    pub fn read_u16(&mut self) -> u16 {
        let value = ((self.buffer[self.cursor] as u16) << 8) | self.buffer[self.cursor + 1] as u16;
        self.cursor += 2;
        value
    }

    pub fn read_string(&mut self) -> &'a str {
        let length = self.read_u16() as usize;
        let start = self.cursor;
        self.cursor += length;
        core::str::from_utf8(&self.buffer[start..self.cursor]).unwrap()
    }

    pub fn read_bytes(&mut self) -> &'a [u8] {
        let length = self.read_u16() as usize;
        self.read_bytes_raw(length)
    }

    pub fn read_bytes_raw(&mut self, length: usize) -> &'a [u8] {
        let start = self.cursor;
        self.cursor += length;
        &self.buffer[start..self.cursor]
    }

    pub fn read_variable_int(&mut self) -> u32 {
        let mut value = 0;
        let mut shift = 0;
        loop {
            let byte = self.buffer[self.cursor];
            self.cursor += 1;
            value |= ((byte & 0x7F) as u32) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
        }
        value
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.cursor
    }

    pub fn skip(&mut self, length: usize) {
        self.cursor += length;
    }

    pub fn mark(&mut self) {
        self.mark = self.cursor;
    }

    pub fn skip_to(&mut self, position: usize) {
        self.cursor = self.mark + position;
    }

    pub fn distance_from_mark(&self) -> usize {
        self.cursor - self.mark
    }
}
