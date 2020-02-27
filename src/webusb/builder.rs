

pub struct DescriptorBuilder<'a> {
    buf: &'a mut [u8],
    position: usize
}

impl DescriptorBuilder<'_> {
    pub fn new(buf: &mut [u8]) -> DescriptorBuilder<'_> {
        DescriptorBuilder{
            buf,
            position: 0
        }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn write(&mut self, bytes: &[u8]) {
        let length = bytes.len();
        self.buf[self.position..self.position + length].copy_from_slice(bytes);
        self.position += length;
    }

    pub fn write_u16(&mut self, val: u16) {
        self.buf[self.position] = (val & 0xFF) as u8;
        self.buf[self.position + 1] = (val >> 8) as u8;
        self.position += 2
    }

    pub fn write_u32(&mut self, val: u32) {
        self.buf[self.position] = (val & 0xFF) as u8;
        self.buf[self.position + 1] = (val >> 8) as u8;
        self.buf[self.position + 2] = (val >> 16) as u8;
        self.buf[self.position + 3] = (val >> 24) as u8;
        self.position += 4
    }

    pub fn write_utf16(&mut self, val: &str) {
        for cp in val.encode_utf16() {
            self.write_u16(cp);
        }
    }

    pub fn buf(&self) -> &[u8] {
        &self.buf[0..self.position]
    }
}
