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
    fn with_capacity(capacity: usize, inner: C) -> Self {
        let buf = vec![0x0; capacity];
        Self {
            inner,
            filled: 0,
            buf: buf.into_boxed_slice(),
        }
    }

    pub(crate) fn get_mut(&mut self) -> &mut C {
        &mut self.inner
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        let min = std::cmp::min(buf.len(), self.filled);
        let (to_fill, rest) = buf.split_at_mut(min);
        to_fill.copy_from_slice(&self.buf[..min]);
        self.consume(min);
        if rest.len() != 0 {
            self.inner.read_exact(&mut rest[..])?;
        }
        Ok(())
    }

    /// Try to read a CRLF terminated line from the underlying reader.
    /// The length of the line is expected to be <= the length of the
    /// internal buffer, suited for reading headers or short responses.
    ///
    /// After reading a line, `consume` should be called.
    fn read_line(&mut self) -> std::io::Result<&str> {
        // check if the buf already has a new line
        if let Some(n) = get_line(&self.buf[..self.filled]) {
            return Ok(std::str::from_utf8(&self.buf[..n]).unwrap());
        }
        loop {
            let (filled, buf) = self.buf.split_at_mut(self.filled);
            if buf.len() == 0 {
                panic!("Ascii protocol response too long");
            }
            let filled = filled.len();
            let read = self.inner.read(&mut buf[..])?;
            if read == 0 {
                panic!("Ascii protocol no line found");
            }
            self.filled += read;
            if let Some(n) = get_line(&buf[..read]) {
                return Ok(std::str::from_utf8(&self.buf[..filled + n]).unwrap());
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
    // before running benches run 'docker run --rm -p 12345:11211 --name my-memcache -d memcached'
    extern crate test;
    use super::MyBufReader;
    use std::{
        io::{Write, BufRead, BufReader},
        net::TcpStream,
    };
    use test::{black_box, Bencher};

    #[bench]
    fn bench_reader(b: &mut Bencher) {
        let stream = TcpStream::connect("127.0.0.1:12345").expect("failed to connect");
        let mut reader = MyBufReader::with_capacity(2 * 1024, stream);
        reader.get_mut().write_all(b"set gpl 00 10000 5 noreply\r\nvalue\r\n").expect("failed to set key");

        // Optionally include some setup
        b.iter(|| {
            // Inner closure, the actual test
            for _ in 0..100 {
                reader
                    .get_mut()
                    .write_all(b"get gpl\r\n")
                    .expect("failed to write");
                let line = reader.read_line().expect("reading line failed");
                let len = line.len();
                reader.consume(len);
            }
        });
    }

    #[bench]
    fn bench_reader2(b: &mut Bencher) {
        let stream = TcpStream::connect("127.0.0.1:12345").expect("failed to connect");
        let mut reader = BufReader::with_capacity(2 * 1024, stream);
        reader.get_mut().write_all(b"set gnu 00 10000 5 noreply\r\nvalue\r\n").expect("failed to set key");
        // Optionally include some setup
        b.iter(|| {
            // Inner closure, the actual test
            for _ in 0..100 {
                reader
                    .get_mut()
                    .write_all(b"get gnu\r\n")
                    .expect("failed to write");
                let mut string = String::new();
                reader.read_line(&mut string).expect("reading line failed");
                black_box(string);
            }
        });
    }
}
