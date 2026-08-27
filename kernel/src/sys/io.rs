pub mod interface {
    pub trait Read {
        type Error;

        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
    }

    pub trait Write {
        type Error;

        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

        fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Self::Error> {
            while !buf.is_empty() {
                let n = self.write(buf)?;

                if n == 0 {
                    break;
                }

                buf = &buf[n..];
            }

            Ok(())
        }
    }
}
