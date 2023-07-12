use crossbeam_queue::SegQueue;
use crossbeam_skiplist::{SkipMap, SkipSet};
use serde::{Serialize, Deserialize};
use std::sync::atomic::AtomicUsize;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

pub type PacketId = Uuid;
pub type ChannelId = Uuid;
pub type ServiceId = u32;

pub trait Randomable {
    fn get_random() -> Self;
}

impl Randomable for Uuid {
    fn get_random() -> Self {
        Uuid::new_v4()
    }
}

impl Randomable for u32 {
    fn get_random() -> Self {
        rand::random()
    }
}

impl Randomable for u8 {
    fn get_random() -> Self {
        rand::random()
    }
}

impl Randomable for i64 {
    fn get_random() -> Self {
        rand::random()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PacketType {
    Pub(String),
    Sub(ServiceId, ChannelId),
    SubAck(PacketId, ServiceId, bool),
    UnSub,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    pub id: PacketId,
    pub wire: ChannelId,
    //immediate sender
    pub from: ServiceId,
    pub p_type: PacketType,
}

impl Packet {
    pub fn copy(&self, sen: ServiceId) -> Packet {
        let mut p = self.clone();
        p.id = self.id;
        p.from = sen;
        p
    }

    pub fn repeat(&self, sen: ServiceId) -> Packet {
        let mut p = self.clone();
        p.id = PacketId::get_random();
        p.from = sen;
        p
    }
}

pub struct RouterInner {
    pub(crate) self_id: ServiceId,
    pub(crate) table: RoutingTable,
    pub(crate) sinks: SkipMap<ServiceId, mpsc::UnboundedSender<Packet>>,
    pub(crate) waiting: SkipMap<ChannelId, RwLock<SegQueue<String>>>,
}

impl RouterInner {
    pub(crate) fn packet(&self, to: ChannelId, msg_type: PacketType) -> Packet {
        Packet {
            id: Randomable::get_random(),
            wire: to,
            from: self.self_id,
            p_type: msg_type,
        }
    }
}

#[derive(Debug, Default)]
pub struct RoutingTable {
    pub channels: SkipSet<ChannelId>,
    pub sub_table: SkipMap<PacketId, ServiceId>,
    pub ack_table: SkipMap<PacketId, AtomicUsize>,
    pub routes: SkipMap<ChannelId, SkipSet<ServiceId>>,
}
