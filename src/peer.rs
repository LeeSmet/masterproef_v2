use futures::{SinkExt, StreamExt};
use std::{
    error::Error,
    net::IpAddr,
    sync::{Arc, RwLock},
};
use tokio::{net::TcpStream, select, sync::mpsc};
use tokio_util::codec::Framed;

use crate::packet::{ControlPacket, ControlStruct, DataPacket};
use crate::{codec::PacketCodec, packet::Packet};

#[derive(Debug, Clone)]
pub struct Peer {
    inner: Arc<RwLock<PeerInner>>,
}

impl Peer {
    pub fn new(
        stream_ip: IpAddr,
        router_data_tx: mpsc::UnboundedSender<DataPacket>,
        router_control_tx: mpsc::UnboundedSender<ControlStruct>,
        stream: TcpStream,
        overlay_ip: IpAddr,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Peer {
            inner: Arc::new(RwLock::new(PeerInner::new(
                stream_ip,
                router_data_tx,
                router_control_tx,
                stream,
                overlay_ip,
            )?)),
        })
    }

    /// Get current sequence number for this peer.
    pub fn hello_seqno(&self) -> u16 {
        self.inner.read().unwrap().hello_seqno
    }

    /// Adds 1 to the sequence number of this peer .
    pub fn increment_hello_seqno(&self) {
        self.inner.write().unwrap().hello_seqno += 1;
    }

    pub fn time_last_received_hello(&self) -> tokio::time::Instant {
        self.inner.read().unwrap().time_last_received_hello
    }

    pub fn set_time_last_received_hello(&self, time: tokio::time::Instant) {
        self.inner.write().unwrap().time_last_received_hello = time
    }

    /// Get overlay IP for this peer
    pub fn overlay_ip(&self) -> IpAddr {
        self.inner.read().unwrap().overlay_ip
    }

    /// For sending data packets towards a peer instance on this node.
    /// It's send over the to_peer_data channel and read from the corresponding receiver.
    /// The receiver sends the packet over the TCP stream towards the destined peer instance on another node
    pub fn send_data_packet(&self, data_packet: DataPacket) -> Result<(), Box<dyn Error>> {
        Ok(self.inner.write().unwrap().to_peer_data.send(data_packet)?)
    }

    /// For sending control packets towards a peer instance on this node.
    /// It's send over the to_peer_control channel and read from the corresponding receiver.
    /// The receiver sends the packet over the TCP stream towards the destined peer instance on another node
    pub fn send_control_packet(&self, control_packet: ControlPacket) -> Result<(), Box<dyn Error>> {
        Ok(self
            .inner
            .write()
            .unwrap()
            .to_peer_control
            .send(control_packet)?)
    }

    pub fn link_cost(&self) -> u16 {
        self.inner.read().unwrap().link_cost
    }

    pub fn set_link_cost(&self, link_cost: u16) {
        self.inner.write().unwrap().link_cost = link_cost
    }

    pub fn underlay_ip(&self) -> IpAddr {
        self.inner.read().unwrap().stream_ip
    }

    pub fn time_last_received_ihu(&self) -> tokio::time::Instant {
        self.inner.read().unwrap().time_last_received_ihu
    }

    pub fn set_time_last_received_ihu(&self, time: tokio::time::Instant) {
        self.inner.write().unwrap().time_last_received_ihu = time
    }
}

impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        self.overlay_ip() == other.overlay_ip()
    }
}

#[derive(Debug)]
struct PeerInner {
    stream_ip: IpAddr,
    to_peer_data: mpsc::UnboundedSender<DataPacket>,
    to_peer_control: mpsc::UnboundedSender<ControlPacket>,
    overlay_ip: IpAddr,
    hello_seqno: u16,
    time_last_received_hello: tokio::time::Instant,
    link_cost: u16,
    time_last_received_ihu: tokio::time::Instant,
}

impl PeerInner {
    pub fn new(
        stream_ip: IpAddr,
        router_data_tx: mpsc::UnboundedSender<DataPacket>,
        router_control_tx: mpsc::UnboundedSender<ControlStruct>,
        stream: TcpStream,
        overlay_ip: IpAddr,
    ) -> Result<Self, Box<dyn Error>> {
        // Framed for peer
        // Used to send and receive packets from a TCP stream
        let mut framed = Framed::new(stream, PacketCodec::new());
        // Data channel for peer
        let (to_peer_data, mut from_routing_data) = mpsc::unbounded_channel::<DataPacket>();
        // Control channel for peer
        let (to_peer_control, mut from_routing_control) =
            mpsc::unbounded_channel::<ControlPacket>();

        // Initialize last_sent_hello_seqno to 0
        let hello_seqno = 0;
        // Initialize last_path_cost to infinity - 1
        let link_cost = u16::MAX - 1;
        // Initialize time_last_received_hello to now
        let time_last_received_hello = tokio::time::Instant::now();
        // Initialiwe time_last_send_ihu
        let time_last_received_ihu = tokio::time::Instant::now();

        // Intialize the timers
        // let ihu_timer = Timer::new_ihu_timer(IHU_INTERVAL);

        tokio::spawn(async move {
            loop {
                select! {
                // Received over the TCP stream
                frame = framed.next() => {
                    match frame {
                        Some(Ok(packet)) => {
                            match packet {
                                Packet::DataPacket(packet) => {
                                    if let Err(error) = router_data_tx.send(packet){
                                        eprintln!("Error sending to to_routing_data: {}", error);
                                    }
                                }
                                Packet::ControlPacket(packet) => {
                                    // Parse the DataPacket into a ControlStruct as the to_routing_control channel expects
                                    let control_struct = ControlStruct {
                                        control_packet: packet,
                                        src_overlay_ip: overlay_ip,
                                        // Note: although this control packet is received from the TCP stream
                                        // we set the src_overlay_ip to the overlay_ip of the peer
                                        // as we 'arrived' in the peer instance of representing the sending node on this current node
                                    };
                                    if let Err(error) = router_control_tx.send(control_struct) {
                                        eprintln!("Error sending to to_routing_control: {}", error);
                                    }

                                }
                            }
                        }
                        Some(Err(e)) => {
                            eprintln!("Error from framed: {}", e);
                        },
                        None => {
                            println!("Stream is closed.");
                            return
                        }
                    }
                }

                Some(packet) = from_routing_data.recv() => {
                    // println!("Sending data packet to peer: {:?}", packet);
                    // Send it over the TCP stream
                    if let Err(e) = framed.send(Packet::DataPacket(packet)).await {
                        eprintln!("Error writing to stream: {}", e);
                    }
                }

                Some(packet) = from_routing_control.recv() => {
                    // Send it over the TCP stream
                    if let Err(e) = framed.send(Packet::ControlPacket(packet)).await {
                        eprintln!("Error writing to stream: {}", e);
                    }
                }
                }
            }
        });
        Ok(Self {
            stream_ip,
            to_peer_data,
            to_peer_control,
            overlay_ip,
            hello_seqno,
            link_cost,
            time_last_received_ihu,
            time_last_received_hello,
        })
    }
}
