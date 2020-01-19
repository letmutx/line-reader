#![feature(test)]
use std::io::Read;

struct MyBufReader<C> {
    inner: C,
    filled: usize,
    buf: Box<[u8]>,
}

fn get_line(buf: &[u8]) -> Option<usize> {
    for (i, r) in buf.iter().enumerate() {
        if *r == b'\r' {
            if buf.get(i + 1) == Some(&b'\n') {
                return Some(i + 2);
            }
        }
    }
    None
}

impl<C: Read> MyBufReader<C> {
    pub fn with_capacity(capacity: usize, inner: C) -> Self {
        let mut buf = Vec::with_capacity(capacity);
        unsafe { buf.set_len(capacity) };
        Self {
            inner,
            filled: 0,
            buf: buf.into_boxed_slice(),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        buf.copy_from_slice(&self.buf[..self.filled]);
        self.filled = 0;
        self.inner.read_exact(&mut buf[self.filled..])
    }

    fn read_line(&mut self) -> std::io::Result<&[u8]> {
        // check if the buf already has a new line
        if let Some(n) = get_line(&self.buf[..self.filled]) {
            return Ok(&self.buf[..n]);
        }
        loop {
            let (filled, buf) = self.buf.split_at_mut(self.filled);
            let filled = filled.len();
            let read = self.inner.read(&mut buf[..])?;
            if read == 0 {
                // TODO: change to error
                panic!("{:?} {:?} line too long", filled, buf.len());
            }
            self.filled += read;
            if let Some(n) = get_line(&buf[..read]) {
                return Ok(&self.buf[..filled + n]);
            }
        }
    }

    fn consume(&mut self, amount: usize) {
        let amount = std::cmp::min(self.filled, amount);
        self.buf.copy_within(amount..self.filled, 0);
        self.filled -= amount;
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use test::{black_box, Bencher};
    use super::MyBufReader;
    use std::io::{BufRead, BufReader};
    use std::fs::File;

    #[bench]
    fn bench_reader(b: &mut Bencher) {
        
        // Optionally include some setup
        b.iter(|| {
            let file = File::open("lines.txt").expect("file not found");
            let mut reader = MyBufReader::with_capacity(2*1024, file);        
            // Inner closure, the actual test
            for _ in 0..100 {
                let line = reader.read_line().expect("reading line failed");
                black_box(line);
                // println!("got line: {}, len: {}", unsafe { std::str::from_utf8_unchecked(&line[..line.len() - 2]) }, line.len());
                let len = line.len();
                reader.consume(len);
            }
        });
    }

//    #[bench]
//    fn bench_reader2(b: &mut Bencher) {
//        // Optionally include some setup
//        b.iter(|| {
//            let file = File::open("lines.txt").expect("file not found");
//            let mut bufreader = BufReader::new(file);
//            // Inner closure, the actual test
//            for _ in 0..100 {
//                let mut string = String::new();
//                bufreader.read_line(&mut string).expect("reading line failed");
//                black_box(string);
//                // println!("got line: {}, len: {}", unsafe { std::str::from_utf8_unchecked(&line[..line.len() - 2]) }, line.len());
//            }
//        });
//    }

}
