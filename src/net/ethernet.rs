pub const ETH_ADDR_LEN: usize = 6;
pub const ETH_HEADER_LEN: usize = 14;

#[allow(dead_code)]
pub const ETHERTYPE_IPV4: u16 = 0x0800;
#[allow(dead_code)]
pub const ETHERTYPE_ARP: u16 = 0x0806;
#[allow(dead_code)]
pub const ETHERTYPE_IPV6: u16 = 0x86DD;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct EthernetHeader {
    pub dst: [u8; ETH_ADDR_LEN],
    pub src: [u8; ETH_ADDR_LEN],
    pub ethertype: u16,
}

pub fn parse_ethernet_frame(data: &[u8]) -> Option<EthernetHeader> {
    if data.len() < ETH_HEADER_LEN {
        return None;
    }

    let mut dst = [0u8; ETH_ADDR_LEN];
    let mut src = [0u8; ETH_ADDR_LEN];

    dst.copy_from_slice(&data[0..6]);
    src.copy_from_slice(&data[6..12]);

    let ethertype = u16::from_be_bytes([data[12], data[13]]);

    Some(EthernetHeader {
        dst,
        src,
        ethertype,
    })
}

#[allow(dead_code)]
pub fn payload(data: &[u8]) -> Option<&[u8]> {
    if data.len() < ETH_HEADER_LEN {
        return None;
    }

    Some(&data[ETH_HEADER_LEN..])
}
