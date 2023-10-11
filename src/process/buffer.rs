pub enum BufData {
    ByteVec(Vec<u8>),
    Buffer(Buffer),
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub data: Vec<u8>,
    size: usize,
    buffer_fill_size: Option<usize>
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer { data: vec![], size: 0, buffer_fill_size: None }
    }
}

impl Buffer {
    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn get_fill_size(&self, size: Option<usize>) -> usize {
        if let Some(size) = size {
            return size;
        } else if let Some(size) = self.buffer_fill_size {
            return size;
        } else {
            return 4096;
        }
    }

    pub fn add(&mut self, data: &mut BufData) {
        match data {
            BufData::ByteVec(data) => {
                self.size += data.len();
                self.data.append(data);
            }
            BufData::Buffer(buf) => {
                self.size += buf.len();
                self.data.append(&mut buf.data);
            }
        }
    }

    pub fn unget(&mut self, data: BufData) -> () {
        match data {
            BufData::ByteVec(data) => {
                self.size += data.len();
                self.data.splice(0..0, data.iter().cloned());
            }
            BufData::Buffer(buf) => {
                self.size += buf.len();
                self.data.splice(0..0, buf.data.iter().cloned());
            }
        }
    }

    pub fn get(&mut self, want: usize) -> Vec<u8> {
        if want >= self.size {
            self.size = 0;
            let data: Vec<u8> = self.data.drain(..).collect();
            return data;
        }
        self.size -= want;
        let data = self.data.drain(0..want).collect();
        data
    }
}
