use std::sync::{atomic::AtomicUsize, Arc};

use actix::prelude::*;
use crossbeam_skiplist::SkipSet;
use log::debug;
use uuid::Uuid;

use crate::{
    handler::MessageHandler,
    r_table::{ChannelId, IncommingPacket, PacketType, RoutingTable},
    transmitter::Payload,
};

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct SubAck {
    pub on: ChannelId,
}

pub struct RouterReciever {
    pub self_rec: Box<dyn MessageHandler<Payload>>,
    inner: Arc<RoutingTable>,
    pub rou_rec: Box<dyn MessageHandler<SubAck>>,
}

impl RouterReciever {
    pub fn new(
        se_re: Box<dyn MessageHandler<Payload>>,
        inn: Arc<RoutingTable>,
        r_rec: Box<dyn MessageHandler<SubAck>>,
    ) -> Self {
        Self {
            self_rec: se_re,
            inner: inn,
            rou_rec: r_rec,
        }
    }

    pub fn message(&self, to: ChannelId, msg_type: PacketType) -> IncommingPacket {
        IncommingPacket {
            id: Uuid::new_v4(),
            wire: to,
            from: self.inner.self_id,
            p_type: msg_type,
        }
    }

    fn handle_pub(&self, msg: &IncommingPacket, payload: &str) {
        if let Some(t) = self.inner.routes.get(&msg.wire) {
            for k in t.value() {
                if *k.value() == msg.from {
                    continue;
                }
                self.inner
                    .sinks
                    .get(&k)
                    .and_then(|f| f.value().try_handle(msg.repeat(self.inner.self_id)).ok());
            }
        }
        if self.inner.channels.contains(&msg.wire) {
            //TODO
            let _ = self.self_rec.try_handle(payload.into());
        }
    }

    fn handle_sub(&self, msg: &IncommingPacket, ch: &Uuid, se: &Uuid) {
        //sub is always passed along
        if self.inner.sinks.len() == 1 {
            if self.inner.channels.contains(ch) {
                //add in routing table
                let val = self.inner.routes.get_or_insert(*ch, SkipSet::new());
                val.value().insert(msg.from);
            }

            //send suback back
            let _ = self
                .inner
                .sinks
                .front()
                .unwrap()
                .value()
                .try_handle(self.message(
                    *ch,
                    PacketType::SubAck(msg.id, *se, self.inner.channels.contains(ch)),
                ));
        } else {
            self.inner.sub_table.insert(msg.id, msg.from);
            let mut act_sent = 0;
            for en in self.inner.sinks.iter() {
                if en.key() == &msg.from {
                    continue;
                }

                let _ = en.value().try_handle(msg.copy(self.inner.self_id));
                act_sent += 1;
            }
            self.inner
                .ack_table
                .insert(msg.id, AtomicUsize::new(act_sent));
        }
    }

    fn handle_sub_ack(&self, msg: &IncommingPacket, id: &Uuid, se: &Uuid, yesno: bool) {
        if *se == self.inner.self_id {
            if yesno {
                let val = self.inner.routes.get_or_insert(msg.wire, SkipSet::new());
                val.value().insert(msg.from);
                debug!("Creating Route to {:?}", val.value().get(&msg.from));
            }
            let yt = self.inner.ack_table.get(id).unwrap();
            yt.value().fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            if yt.value().load(std::sync::atomic::Ordering::SeqCst) == 0 {
                self.inner.sub_table.remove(id);
                //send subrelease message
                let _ = self.rou_rec.try_handle(SubAck { on: msg.wire });
            }
        } else if yesno || self.inner.channels.contains(&msg.wire) {
            let vs = self.inner.routes.get_or_insert(msg.wire, SkipSet::new());

            if yesno {
                vs.value().insert(msg.from);
            }

            //send earliest sub response that is true
            if let Some(ent) = self.inner.ack_table.remove(id) {
                let backroute = self.inner.sub_table.remove(id).unwrap();

                vs.value().insert(*backroute.value());

                let _ = self
                    .inner
                    .sinks
                    .get(backroute.value())
                    .unwrap()
                    .value()
                    .try_handle(self.message(msg.wire, PacketType::SubAck(*id, *se, true)));

                ent.value()
                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        } else if let Some(ent) = self.inner.ack_table.get(id) {
            ent.value()
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            if ent.value().load(std::sync::atomic::Ordering::SeqCst) == 0 {
                ent.remove();
                let backroute = self.inner.sub_table.remove(id).unwrap();
                let _ = self
                    .inner
                    .sinks
                    .get(backroute.value())
                    .unwrap()
                    .value()
                    .try_handle(self.message(msg.wire, PacketType::SubAck(*id, *se, false)));
            }
        }
    }

    fn handle_unsub(&self, msg: &IncommingPacket) {
        //if you see an unsub that means you look at router table and invalidate that route
        let y = self.inner.routes.get(&msg.wire).unwrap();
        if y.value().len() > 1 || self.inner.channels.contains(&msg.wire) {
            //means kill unsub at this node
        } else {
            //propogate unusub and kill entry in routing table
            for i in &self.inner.sinks {
                if *i.key() == msg.from {
                    continue;
                }
                let _ = i.value().try_handle(msg.copy(self.inner.self_id));
            }
        }
        y.value().remove(&msg.from);
    }

    fn handle_packet(&self, msg: &IncommingPacket) {
        debug!("{} {msg:?}", self.inner.self_id);
        //handle based on type
        //if msg is pub and no entry in rounting table or is not self drop message
        match &msg.p_type {
            PacketType::Pub(payload) => {
                self.handle_pub(&msg, payload);
            }
            PacketType::Sub(se, ch) => {
                self.handle_sub(&msg, ch, se);
            }
            PacketType::SubAck(id, se, yesno) => {
                self.handle_sub_ack(&msg, id, se, *yesno);
            }
            PacketType::UnSub => {
                self.handle_unsub(&msg);
            }
        };
    }
}

impl Actor for RouterReciever {
    type Context = Context<Self>;
}

impl Handler<IncommingPacket> for RouterReciever {
    type Result = ();

    fn handle(&mut self, msg: IncommingPacket, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_packet(&msg);
    }
}
