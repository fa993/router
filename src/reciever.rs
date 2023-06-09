use std::sync::{atomic::AtomicUsize, Arc, RwLock};

use actix::prelude::*;
use crossbeam_queue::SegQueue;
use crossbeam_skiplist::{SkipMap, SkipSet};
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
    waiting: SkipMap<ChannelId, RwLock<SegQueue<String>>>,
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
            waiting: SkipMap::default(),
        }
    }
}

impl RouterReciever {
    fn packet(&self, to: ChannelId, msg_type: PacketType) -> IncommingPacket {
        IncommingPacket {
            id: Uuid::new_v4(),
            wire: to,
            from: self.inner.self_id,
            p_type: msg_type,
        }
    }

    fn handle_pub_packet(&self, msg: &IncommingPacket, payload: &str) {
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

    fn handle_sub_packet(&self, msg: &IncommingPacket, ch: &Uuid, se: &Uuid) {
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
                .try_handle(self.packet(
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

    fn handle_sub_ack_packet(&self, msg: &IncommingPacket, id: &Uuid, se: &Uuid, yesno: bool) {
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
                    .try_handle(self.packet(msg.wire, PacketType::SubAck(*id, *se, true)));

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
                    .try_handle(self.packet(msg.wire, PacketType::SubAck(*id, *se, false)));
            }
        }
    }

    fn handle_unsub_packet(&self, msg: &IncommingPacket) {
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
                self.handle_pub_packet(msg, payload);
            }
            PacketType::Sub(se, ch) => {
                self.handle_sub_packet(msg, ch, se);
            }
            PacketType::SubAck(id, se, yesno) => {
                self.handle_sub_ack_packet(msg, id, se, *yesno);
            }
            PacketType::UnSub => {
                self.handle_unsub_packet(msg);
            }
        };
    }
}

impl RouterReciever {
    pub fn handle_pub_msg(&mut self, to: ChannelId, payload: &str) {
        if let Some(t) = self.waiting.get(&to) {
            debug!("Pushing to Publish to Queue to: {to} payload:{payload}");
            let _ = t.value().read().map(|f| {
                f.push(payload.to_string());
            });
        } else if let Some(t) = self.inner.routes.get(&to) {
            debug!("Publishing to: {to} payload:{payload}");
            for k in t.value() {
                let s = self.inner.sinks.get(k.value()).unwrap();
                let _ = s
                    .value()
                    .try_handle(self.packet(to, PacketType::Pub(payload.to_string())));
            }
        }
    }

    pub fn handle_unsub_msg(&mut self, to: ChannelId) {
        //if we have 1 routing entry for that channel it means we are leaf.. that means we actually send unsub message
        //if we have more than one... it means we are not leaf... in that case just remove from channel and wait
        debug!("Unsubbing from {}", to);
        self.inner.channels.remove(&to);

        if let Some(t) = self.inner.routes.get(&to) {
            if t.value().len() == 1 {
                //propogate unsub and remove entry
                // self.tx.send()
                let val = self
                    .inner
                    .sinks
                    .get(t.value().front().unwrap().value())
                    .unwrap();
                let p = self.packet(to, PacketType::UnSub);

                debug!("Actually propogating unsub {p:?}");
                let _ = val.value().try_handle(p);
            }
        }
    }

    pub fn handle_sub_msg(&mut self, to: ChannelId) {
        self.inner.channels.insert(to);
        self.waiting.insert(to, RwLock::new(SegQueue::default()));

        let sb = self.packet(to, PacketType::Sub(self.inner.self_id, to));

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
