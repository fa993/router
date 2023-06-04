use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, AtomicUsize},
        Arc,
    },
};

use actix::prelude::*;
use uuid::Uuid;

use crate::{
    r_table::{ChannelId, Packet, PacketType, RoutingTable},
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

#[derive(Debug)]
pub struct PubMsg {
    pub to: ChannelId,
    pub payload: Payload,
}

impl Message for Payload {
    type Result = ();
}

impl Message for PubMsg {
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
            println!("Waiting");
            t.push(msg);
        } else if let Some(t) = self.inner.routes.get(&msg.to) {
            println!("Not Waiting");
            for k in t.value() {
                // if self.inner.self_id == 3 {
                //     println!("pub loop {:?}", k);
                // }
                let s = self.inner.sinks.get(k.value()).unwrap();
                let _ = s.value().try_handle(Packet {
                    id: Uuid::new_v4(),
                    wire: msg.to,
                    from: self.inner.self_id,
                    p_type: PacketType::Pub(msg.payload.msg.clone()),
                });
            }
        }
        println!("Nothing");
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
        let val = self.inner.channels.get_or_insert(msg.on, AtomicU32::new(0));

        val.value()
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        self.waiting.insert(msg.on, Vec::new());

        let sb = Packet {
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
        println!("asddsadsads");
        self.waiting
            .remove(&msg.on)
            .map_or(Vec::new(), |f| f)
            .into_iter()
            .for_each(|f| {
                let _ = ctx.address().try_send(f);
            });
    }
}
