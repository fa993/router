use std::sync::{atomic::AtomicUsize, Arc};

use crate::handler::PacketHandler;

use actix::Message;
use crossbeam_skiplist::{SkipMap, SkipSet};
use uuid::Uuid;

pub type PacketId = Uuid;
pub type ChannelId = Uuid;
pub type ServiceId = Uuid;

#[derive(Debug, Clone)]
pub enum PacketType {
    Pub(String),
    Sub(ServiceId, ChannelId),
    SubAck(PacketId, ServiceId, bool),
    UnSub,
}

#[derive(Debug, Clone)]
pub struct IncommingPacket {
    pub id: PacketId,
    pub wire: ChannelId,
    //immediate sender
    pub from: ServiceId,
    pub p_type: PacketType,
}

impl IncommingPacket {
    pub fn copy(&self, sen: ServiceId) -> IncommingPacket {
        let mut p = self.clone();
        p.id = self.id;
        p.from = sen;
        p
    }

    pub fn repeat(&self, sen: ServiceId) -> IncommingPacket {
        let mut p = self.clone();
        p.id = Uuid::new_v4();
        p.from = sen;
        p
    }
}

impl Message for IncommingPacket {
    type Result = ();
}

#[derive(Debug, Default)]
pub struct RoutingTable {
    pub self_id: ServiceId,
    pub sinks: SkipMap<ServiceId, Arc<PacketHandler>>,
    pub channels: SkipSet<ChannelId>,
    pub sub_table: SkipMap<PacketId, ServiceId>,
    pub ack_table: SkipMap<PacketId, AtomicUsize>,
    pub routes: SkipMap<ChannelId, SkipSet<ServiceId>>,
}

impl RoutingTable {
    pub fn new() -> RoutingTable {
        Self::with_id(Uuid::new_v4())
        // Self::with_id(rand::random())
    }

    pub fn with_id(self_id: ServiceId) -> RoutingTable {
        Self {
            self_id,
            ..Default::default()
        }
    }
}
