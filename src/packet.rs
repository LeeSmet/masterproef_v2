use std::net::{IpAddr, Ipv4Addr};

/* ********************************PAKCET*********************************** */
#[derive(Debug, Clone)]
pub enum Packet {
    DataPacket(DataPacket),
    ControlPacket(ControlPacket),
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PacketType {
    DataPacket = 0,
    ControlPacket = 1,
}

/* ******************************DATA PACKET********************************* */
#[derive(Debug, Clone)]
pub struct DataPacket {
    pub raw_data: Vec<u8>,
    pub dest_ip: Ipv4Addr,
}

impl DataPacket {
    pub fn get_dest_ip(&self) -> Ipv4Addr {
        self.dest_ip
    }
}

/* ****************************CONTROL PACKET******************************** */
#[derive(Debug, PartialEq, Clone)]
pub struct ControlPacket {
    pub message_type: ControlPacketType,
    pub body_length: u8,
    pub body: Option<ControlPacketBody>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ControlPacketType {
    Pad1 = 0,
    PadN = 1,
    AckReq = 2,
    Ack = 3,
    Hello = 4,
    IHU = 5,
    RouterID = 6,
    NextHop = 7,
    Update = 8,
    RouteReq = 9,
    SeqnoReq = 10,
}

impl ControlPacketType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Pad1),
            1 => Some(Self::PadN),
            2 => Some(Self::AckReq),
            3 => Some(Self::Ack),
            4 => Some(Self::Hello),
            5 => Some(Self::IHU),
            6 => Some(Self::RouterID),
            7 => Some(Self::NextHop),
            8 => Some(Self::Update),
            9 => Some(Self::RouteReq),
            10 => Some(Self::SeqnoReq),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ControlPacketBody {
    Pad1,
    PadN(u8),
    AckReq { nonce: u16, interval: u16 },
    Ack { nonce: u16 },
    Hello { 
        flags: u16,
        seqno: u16, 
        interval: u16,
    },
    IHU { 
        address_encoding: u8,
        reserved: u8,
        rxcost: u16, 
        interval: u16,
        address: IpAddr, 
    },
    RouterID { router_id: u16 },
    NextHop { address: IpAddr },
    Update {
        address: IpAddr,
        prefix: u8,
        seqno: u16,
        metric: u16,
        interval: u16,
    },
    RouteReq { prefix: IpAddr, plen: u8 },
    SeqnoReq {
        prefix: IpAddr,
        plen: u8,
        seqno: u16,
        router_id: u16,
    },
}