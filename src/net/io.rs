use super::{NetError, NetResult};

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> NetResult<usize>;

    fn read_exact(&mut self, mut buf: &mut [u8]) -> NetResult<()> {
        while !buf.is_empty() {
            let count = self.read(buf)?;

            if count == 0 {
                return Err(NetError::Closed);
            }

            let (_, remaining) = buf.split_at_mut(count);
            buf = remaining;
        }

        Ok(())
    }
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> NetResult<usize>;

    fn write_all(&mut self, mut buf: &[u8]) -> NetResult<()> {
        while !buf.is_empty() {
            let count = self.write(buf)?;

            if count == 0 {
                return Err(NetError::ConnectionReset);
            }

            buf = &buf[count..];
        }

        Ok(())
    }

    fn flush(&mut self) -> NetResult<()> {
        Ok(())
    }
}
