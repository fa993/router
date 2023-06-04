use std::{collections::HashMap, sync::Arc, error::Error};

use actix::prelude::*;
use uuid::Uuid;

use crate::{
    handler::{ActixPacketRecipient, ActixRecipient, PacketHandler},
    r_table::{ChannelId, RoutingTable, ServiceId},
    reciever::RouterReciever,
    transmitter::{Payload, PubMsg, RouterTransmitter, SubMsg},
};

pub struct Router {
    src: Arc<RoutingTable>,
    tx: Addr<RouterTransmitter>,
    rx: Addr<RouterReciever>,
}

impl Router {
    pub fn new_test(c: u32) -> Router {
        let rt = Arc::new(RoutingTable::with_id(Uuid::new_v4()));
        // let rt = Arc::new(RoutingTable::with_id(c));

        let ad = Printer { c }.start();
        let add = ad.recipient();

        let rou = RouterTransmitter {
            inner: rt.clone(),
            waiting: HashMap::default(),
        };
        let ar = rou.start();
        let ar2 = ar.clone();

        let rts = RouterReciever {
            self_rec: Box::new(ActixRecipient::new(add)),
            inner: rt.clone(),
            rou_rec: Box::new(ActixRecipient::new(ar.recipient())),
        };
        Self {
            src: rt,
            tx: ar2,
            rx: rts.start(),
        }
    }

    pub async fn sub(&self, on: ChannelId) -> Result<(), Box<dyn Error>> {
        self.tx.send(SubMsg { on }).await.map_err(|f| f.into())
    }

    pub async fn publish(&self, payload: &str, on: ChannelId) -> Result<(), Box<dyn Error>> {
        self.tx
            .send(PubMsg {
                to: on,
                payload: payload.into(),
            })
            .await
            .map_err(|f| f.into())
    }

    pub fn add_entry(&self, for_service: ServiceId, handler: Arc<PacketHandler>) {
        self.src.sinks.insert(for_service, handler);
    }

    pub fn id(&self) -> ServiceId {
        self.src.self_id
    }

    pub fn create_handler(&self) -> Arc<PacketHandler> {
        Arc::new(ActixPacketRecipient::new(self.rx.clone().recipient()))
    }
}

struct Printer {
    c: u32,
}

impl Actor for Printer {
    type Context = Context<Self>;
}

impl Handler<Payload> for Printer {
    type Result = ();
    fn handle(&mut self, msg: Payload, _ctx: &mut Self::Context) -> Self::Result {
        println!("{} got {:?}", self.c, msg);
    }
}

// pub struct RoutingLayer;

// impl RoutingLayer {
//     pub fn new(
//         c: u32,
//     ) -> (
//         Arc<RoutingTable>,
//         Addr<RouterReciever>,
//         Addr<RouterTransmitter>,
//     ) {
//         let rt = Arc::new(RoutingTable::with_id(Uuid::new_v4()));
//         // let rt = Arc::new(RoutingTable::with_id(c));

//         let ad = Printer { c }.start();
//         let add = ad.recipient();

//         let rou = RouterTransmitter {
//             inner: rt.clone(),
//             waiting: HashMap::default(),
//         };
//         let ar = rou.start();
//         let ar2 = ar.clone();

//         let rts = RouterReciever {
//             self_rec: add,
//             inner: rt.clone(),
//             rou_rec: ar.recipient(),
//         };

//         (rt, rts.start(), ar2)
//     }
// }
