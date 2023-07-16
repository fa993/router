use std::fmt::Display;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crossbeam_skiplist::{SkipMap, SkipSet};
use log::debug;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::r_table::{ChannelId, Packet, PacketId, PacketType, RoutingTable};
use crate::r_table::{RouterInner, ServiceId};
use crate::transmitter::RouterTx;

#[derive(Debug, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Payload {
    pub dest: ChannelId,
    pub contents: String
}

impl Payload {
    pub fn new(to: ChannelId, body: String) -> Payload {
        Self { dest: to, contents: body }
    }
}

impl Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}):{}", self.dest, self.contents)
    }
}

pub enum OutgoingMessage {
    Pub(Payload),
    Sub(ChannelId),
    Unsub(ChannelId),
}

pub struct RouterRx {
    inner: Arc<RouterInner>,
    self_rec: mpsc::UnboundedSender<Payload>,
    pck_stream: mpsc::UnboundedReceiver<Packet>,
}

impl RouterRx {
    pub fn new(
        self_id: ServiceId,
        inn: RoutingTable,
        se_re: mpsc::UnboundedSender<Payload>,
    ) -> (Self, mpsc::UnboundedSender<Packet>) {
        let (ptx, prx) = mpsc::unbounded_channel();
        (
            Self {
                self_rec: se_re,
                inner: Arc::new(RouterInner {
                    self_id,
                    sinks: SkipMap::default(),
                    table: inn,
                    waiting: SkipMap::default(),
                }),
                pck_stream: prx,
            },
            ptx,
        )
    }

    pub fn with_inner(
        inn: Arc<RouterInner>,
        se_re: mpsc::UnboundedSender<Payload>,
        prx: UnboundedReceiver<Packet>,
    ) -> Self {
        Self {
            self_rec: se_re,
            inner: inn,
            pck_stream: prx,
        }
    }

    pub fn inner(&self) -> Arc<RouterInner> {
        self.inner.clone()
    }

    pub fn add_entry(&self, for_service: ServiceId, handler: mpsc::UnboundedSender<Packet>) {
        self.inner.sinks.insert(for_service, handler);
    }

    pub async fn recv_packets(&mut self) {
        while let Some(t) = self.pck_stream.recv().await {
            self.handle_packet(&t).await;
        }
    }

    pub fn create_tx(&self) -> RouterTx {
        RouterTx {
            inner: self.inner.clone(),
        }
    }
}

impl RouterRx {
    fn handle_pub_packet(&self, msg: &Packet, payload: &str) {
        if let Some(t) = self.inner.table.routes.get(&msg.wire) {
            for k in t.value() {
                if *k.value() == msg.from {
                    continue;
                }
                self.inner
                    .sinks
                    .get(&k)
                    .and_then(|f| f.value().send(msg.repeat(self.inner.self_id)).ok());
            }
        }
        if self.inner.table.channels.contains(&msg.wire) {
            //TODO
            let _ = self.self_rec.send(Payload { dest: msg.wire, contents: payload.into() });
        }
    }

    fn handle_sub_packet(&self, msg: &Packet, ch: &ChannelId, se: &ServiceId) {
        //sub is always passed along
        if self.inner.sinks.len() == 1 {
            if self.inner.table.channels.contains(ch) {
                //add in routing table
                let val = self.inner.table.routes.get_or_insert(*ch, SkipSet::new());
                val.value().insert(msg.from);
                debug!("Creating Route to {:?}", val.value().get(&msg.from));
            }

            //send suback back
            let _ = self
                .inner
                .sinks
                .front()
                .unwrap()
                .value()
                .send(self.inner.packet(
                    *ch,
                    PacketType::SubAck(msg.id, *se, self.inner.table.channels.contains(ch)),
                ));
        } else {
            self.inner.table.sub_table.insert(msg.id, msg.from);
            let act_sent = SkipSet::new();
            for en in self.inner.sinks.iter() {
                if en.key() == &msg.from {
                    continue;
                }

                let _ = en.value().send(msg.copy(self.inner.self_id));
                act_sent.insert(*en.key());
            }
            self.inner
                .table
                .ack_table
                .insert(msg.id, act_sent);
        }
    }

    async fn handle_sub_ack_packet(
        &self,
        msg: &Packet,
        id: &PacketId,
        se: &ServiceId,
        yesno: bool,
    ) {
        if *se == self.inner.self_id {
            if yesno {
                let val = self
                    .inner
                    .table
                    .routes
                    .get_or_insert(msg.wire, SkipSet::new());
                val.value().insert(msg.from);
                debug!("Creating Route to {:?}", val.value().get(&msg.from));
            }
            let yt = self.inner.table.ack_table.get(id).unwrap();
            yt.value().remove(&msg.from);
            if yt.value().is_empty() {
                self.inner.table.sub_table.remove(id);
                //send subrelease message
                self.send_sub_ack_msg(msg.wire).await;
            }
        } else if yesno || self.inner.table.channels.contains(&msg.wire) {
            let vs = self
                .inner
                .table
                .routes
                .get_or_insert(msg.wire, SkipSet::new());

            if yesno {
                vs.value().insert(msg.from);
            }

            //send earliest sub response that is true
            if self.inner.table.ack_table.remove(id).is_some() {
                let backroute = self.inner.table.sub_table.remove(id).unwrap();

                vs.value().insert(*backroute.value());

                let _ = self
                    .inner
                    .sinks
                    .get(backroute.value())
                    .unwrap()
                    .value()
                    .send(
                        self.inner
                            .packet(msg.wire, PacketType::SubAck(*id, *se, true)),
                    );

                //no use for this statement this has been removed
                // ent.value()
                //     .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        } else if let Some(ent) = self.inner.table.ack_table.get(id) {
            ent.value()
                .remove(&msg.from);
            if ent.value().is_empty() {
                ent.remove();
                let backroute = self.inner.table.sub_table.remove(id).unwrap();
                let _ = self
                    .inner
                    .sinks
                    .get(backroute.value())
                    .unwrap()
                    .value()
                    .send(
                        self.inner
                            .packet(msg.wire, PacketType::SubAck(*id, *se, false)),
                    );
            }
        }
    }

    fn handle_unsub_packet(&self, msg: &Packet) {
        //if you see an unsub that means you look at router table and invalidate that route
        let y = self.inner.table.routes.get(&msg.wire).unwrap();
        if y.value().len() > 1 || self.inner.table.channels.contains(&msg.wire) {
            //means kill unsub at this node
        } else {
            //propogate unusub and kill entry in routing table
            for i in self.inner.sinks.iter() {
                if *i.key() == msg.from {
                    continue;
                }
                let _ = i.value().send(msg.copy(self.inner.self_id));
            }
        }
        y.value().remove(&msg.from);
    }

    async fn handle_packet(&self, msg: &Packet) {
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
                self.handle_sub_ack_packet(msg, id, se, *yesno).await;
            }
            PacketType::UnSub => {
                self.handle_unsub_packet(msg);
            }
        };
    }

    async fn send_sub_ack_msg(&self, to: ChannelId) {
        debug!("Releasing leftover pub packets to:{to}");
        let en = self.inner.waiting.remove(&to);
        let mut msgs = Vec::new();
        if let Some(t) = en {
            let r = t.value().write().await;
            while let Some(f) = r.pop() {
                debug!("Extracting All accumulated payloads to buffer payload: {f} to: {to}");
                msgs.push(f);
            }
        }

        let rous = self.inner.table.routes.get(&to);
        if let Some(ro) = rous {
            debug!("Publishing All accumulated to: {to}");
            for i in ro.value() {
                if let Some(s) = self.inner.sinks.get(i.value()) {
                    msgs.iter().for_each(|f| {
                        let _ = s
                            .value()
                            .send(self.inner.packet(to, PacketType::Pub(f.clone())));
                    });
                }
            }
        }
    }
}
