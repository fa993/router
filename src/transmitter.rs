use std::sync::{atomic::AtomicUsize, Arc};

use crossbeam_queue::SegQueue;
use log::debug;
use tokio::sync::RwLock;

use crate::{
    r_table::{ChannelId, Packet, PacketId, PacketType, Randomable, RouterInner},
    reciever::OutgoingMessage,
};

#[derive(Clone)]
pub struct RouterTx {
    pub(crate) inner: Arc<RouterInner>,
}

impl RouterTx {
    pub async fn handle_msg(&self, t: OutgoingMessage) {
        match t {
            OutgoingMessage::Pub(f, t) => self.send_pub_msg(t, &f).await,
            OutgoingMessage::Sub(t) => self.send_sub_msg(t),
            OutgoingMessage::Unsub(t) => self.send_unsub_msg(t),
        }
    }

    fn packet(&self, to: ChannelId, msg_type: PacketType) -> Packet {
        Packet {
            id: PacketId::get_random(),
            wire: to,
            from: self.inner.self_id,
            p_type: msg_type,
        }
    }

    pub async fn send_pub_msg(&self, to: ChannelId, payload: &str) {
        if let Some(t) = self.inner.waiting.get(&to) {
            debug!("Pushing to Publish Queue to: {to} payload:{payload}");
            let qu = t.value().read().await;
            qu.push(payload.to_string());
        } else if let Some(t) = self.inner.table.routes.get(&to) {
            debug!("Publishing to: {to} payload:{payload}");
            for k in t.value() {
                let s = self.inner.sinks.get(k.value()).unwrap();
                let _ = s
                    .value()
                    .send(self.packet(to, PacketType::Pub(payload.to_string())));
            }
        }
    }

    pub fn send_unsub_msg(&self, to: ChannelId) {
        //if we have 1 routing entry for that channel it means we are leaf.. that means we actually send unsub message
        //if we have more than one... it means we are not leaf... in that case just remove from channel and wait
        debug!("Unsubbing from {}", to);
        self.inner.table.channels.remove(&to);

        if let Some(t) = self.inner.table.routes.get(&to) {
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
                let _ = val.value().send(p);
            }
        }
    }

    pub fn send_sub_msg(&self, to: ChannelId) {
        self.inner.table.channels.insert(to);
        self.inner
            .waiting
            .insert(to, RwLock::new(SegQueue::default()));

        let sb = self.packet(to, PacketType::Sub(self.inner.self_id, to));

        //construct sub packet then propogate
        let mut act_sent = 0;
        for yt in self.inner.sinks.iter() {
            let _ = yt.value().send(sb.clone());
            act_sent += 1;
        }

        self.inner.table.sub_table.insert(sb.id, self.inner.self_id);
        self.inner
            .table
            .ack_table
            .insert(sb.id, AtomicUsize::new(act_sent));
    }
}
