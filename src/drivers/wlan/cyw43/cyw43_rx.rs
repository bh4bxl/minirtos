use crate::{drivers::wlan::cyw43::cyw43_sdpcm::SdpcmPacket, sys::device_driver::DevError};

use super::{Cyw43Inner, cyw43_bus::Func, cyw43_regs::*};

impl Cyw43Inner {
    pub(super) fn poll(&mut self) -> Result<(), DevError> {
        loop {
            let status = self.bus.read_reg::<u32>(Func::Bus, SPI_STATUS_REGISTER)?;

            if status & STATUS_F2_PKT_AVAILABLE == 0 {
                //defmt::warn!("CYW43: F2 packet not available 0x{:08x}", status);
                break;
            }

            defmt::debug!("CYW43: F2 packet available status=0x{:08x}", status);

            self.read_wlan_frame()?;
        }
        Ok(())
    }

    fn read_wlan_frame(&mut self) -> Result<(), DevError> {
        let read_len = self.spid_buf.len() - 16;

        let len = self
            .bus
            .read_bytes(Func::Wlan, 0, read_len, &mut self.spid_buf)?;

        let frame = &self.spid_buf[..len];

        let size = u16::from_le_bytes([frame[0], frame[1]]) as usize;

        if size == 0 {
            defmt::warn!("CYW43: empty F2 frame");
            return Ok(());
        }

        if size > len {
            defmt::warn!("CYW43: invalid size {} > read {}", size, len);
            return Ok(());
        }

        match self.sdpcm_process_rx_packet(size)? {
            SdpcmPacket::AsyncEvent { offset, len } => {
                self.handle_async_event(offset, len)?;
            }

            SdpcmPacket::Data {
                offset,
                len,
                interface,
            } => {
                defmt::info!(
                    "CYW43: [DATA] offset={} len={} iface={}",
                    offset,
                    len,
                    interface
                );
            }

            SdpcmPacket::Control { .. } => {}

            SdpcmPacket::None => {}

            SdpcmPacket::Unexpected(ch) => {
                defmt::warn!("CYW43: [RX] unexpected channel {}", ch);
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn hand_rx_frame(&mut self) -> Result<(), DevError> {
        Ok(())
    }

    #[allow(dead_code)]
    fn handle_event(&mut self) -> Result<(), DevError> {
        Ok(())
    }
}
