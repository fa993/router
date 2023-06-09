use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

use actix::prelude::*;
use log::debug;
use uuid::Uuid;

use crate::{
    r_table::{ChannelId, IncommingPacket, PacketType, RoutingTable},
    reciever::SubAck,
};

#[derive(Debug, Clone)]
pub struct Payload {
    pub msg: String,
}

impl From<String> for Payload {
    fn from(value: String) -> Self {
        Self { msg: value }
    }
}

impl From<&String> for Payload {
    fn from(value: &String) -> Self {
        Self { msg: value.clone() }
    }
}

impl From<&str> for Payload {
    fn from(value: &str) -> Self {
        Self {
            msg: value.to_string(),
        }
    }
}

impl Message for Payload {
    type Result = ();
}

#[derive(Debug)]
pub struct PubMsg {
    pub to: ChannelId,
    pub payload: Payload,
}

impl Message for PubMsg {
    type Result = ();
}

#[derive(Debug)]
pub struct UnsubMsg {
    pub on: ChannelId,
}

impl Message for UnsubMsg {
    type Result = ();
}

pub struct RouterTransmitter {
    pub inner: Arc<RoutingTable>,
    pub(crate) waiting: HashMap<ChannelId, Vec<PubMsg>>,
}

impl Actor for RouterTransmitter {
    type Context = Context<Self>;
}

impl Handler<PubMsg> for RouterTransmitter {
    type Result = ();

    fn handle(&mut self, msg: PubMsg, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(t) = self.waiting.get_mut(&msg.to) {
            debug!("Pushing to Publish to Queue {msg:?}");
            t.push(msg);
        } else if let Some(t) = self.inner.routes.get(&msg.to) {
            debug!("Publishing {msg:?}");
            for k in t.value() {
                let s = self.inner.sinks.get(k.value()).unwrap();
                let _ = s.value().try_handle(IncommingPacket {
                    id: Uuid::new_v4(),
                    wire: msg.to,
                    from: self.inner.self_id,
                    p_type: PacketType::Pub(msg.payload.msg.clone()),
                });
            }
        }
    }
}

impl Handler<UnsubMsg> for RouterTransmitter {
    type Result = ();

    fn handle(&mut self, msg: UnsubMsg, _ctx: &mut Self::Context) -> Self::Result {
        //if we have 1 routing entry for that channel it means we are leaf.. that means we actually send unsub message
        //if we have more than one... it means we are not leaf... in that case just remove from channel and wait
        debug!("Unsubbing from {}", msg.on);
        self.inner.channels.remove(&msg.on);

        if let Some(t) = self.inner.routes.get(&msg.on) {
            if t.value().len() == 1 {
                //propogate unsub and remove entry
                // self.tx.send()
                let val = self
                    .inner
                    .sinks
                    .get(t.value().front().unwrap().value())
                    .unwrap();
                let p = IncommingPacket {
                    from: self.inner.self_id.clone(),
                    id: Uuid::new_v4(),
                    p_type: PacketType::UnSub,
                    wire: msg.on,
                };
                debug!("Actually propogating unsub {p:?}");
                let _ = val.value().try_handle(p);
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SubMsg {
    pub on: ChannelId,
}

impl From<ChannelId> for SubMsg {
    fn from(value: ChannelId) -> Self {
        Self { on: value }
    }
}

impl Handler<SubMsg> for RouterTransmitter {
    type Result = ();

    fn handle(&mut self, msg: SubMsg, _ctx: &mut Self::Context) -> Self::Result {
        self.inner.channels.insert(msg.on);
        self.waiting.insert(msg.on, Vec::new());

        let sb = IncommingPacket {
            from: self.inner.self_id,
            id: Uuid::new_v4(),
            p_type: PacketType::Sub(self.inner.self_id, msg.on),
            wire: msg.on,
        };

        //construct sub packet then propogate
        let mut act_sent = 0;
        for yt in self.inner.sinks.iter() {
            let _ = yt.value().try_handle(sb.clone());
            act_sent += 1;
        }

        self.inner.sub_table.insert(sb.id, self.inner.self_id);
        self.inner
            .ack_table
            .insert(sb.id, AtomicUsize::new(act_sent));
        //construct sub
    }
}

impl Handler<SubAck> for RouterTransmitter {
    type Result = ();

    fn handle(&mut self, msg: SubAck, ctx: &mut Self::Context) -> Self::Result {
        debug!("Propogating SubAck Upstream {msg:?}");
        self.waiting
            .remove(&msg.on)
            .map_or(Vec::new(), |f| f)
            .into_iter()
            .for_each(|f| {
                let _ = ctx.address().try_send(f);
            });
    }
}
