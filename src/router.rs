use tokio::sync::mpsc::{self, UnboundedSender};

use crate::{
    r_table::{Packet, RoutingTable},
    reciever::RouterRx,
    transmitter::RouterTx,
};

pub struct Router {
    pub ptx: UnboundedSender<Packet>,
    pub sender: RouterTx,
}

impl Router {
    // pub fn new<T>(c: u32, t: T) where T: Future + Send + 'static {

    // }

    pub fn new_test(c: u32) -> Router {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(t) = rx.recv().await {
                println!("{c} got payload {t}");
            }
        });
        let (mut rr, ptx) = RouterRx::new(c, RoutingTable::default(), tx);
        let rtx = rr.create_tx();

        tokio::spawn(async move {
            rr.recv_packets().await;
        });

        Self { ptx, sender: rtx }
    }

    //this is both ways
    pub fn connect_to(&self, other: &Router) {
        self.sender
            .inner()
            .sinks
            .insert(other.sender.inner().self_id.clone(), other.ptx.clone());
        other
            .sender
            .inner()
            .sinks
            .insert(self.sender.inner().self_id.clone(), self.ptx.clone());

        // println!("{:?}", self.rx.inner().sinks.len())
    }

    // pub fn publish(&self, wire: ChannelId, payload: String) {
    //     let _ = self.send(OutgoingMessage::Pub(payload, wire));
    // }

    // pub fn sub(&self, wire: ChannelId) {
    //     let _ = self.sender.send(OutgoingMessage::Sub(wire));
    // }

    // pub fn unsub(&self, wire: ChannelId) {
    //     let _ = self.sender.send(OutgoingMessage::Unsub(wire));
    // }
}

// pub fn setup_testing_router(c: u32) -> (mpsc::Sender<OutgoingMessage>) {
//     let (tx, mut rx) = mpsc::unbounded_channel();
//     tokio::spawn(async move {
//         while let Some(t) = rx.recv().await {
//             println!("{c} got payload {t}");
//         }
//     });
//     let (rr, ptx) = RouterRx::new(ChannelId::get_random(), RoutingTable::default(), tx);

//     todo!()

// }

// pub struct Router {
//     src: Arc<RoutingTable>,
//     tx: Addr<RouterTransmitter>,
//     rx: Addr<RouterReciever>,
// }

// impl Router {
//     pub fn new_test(c: u32) -> Router {
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

//         let rts = RouterReciever::new(
//             Box::new(ActixRecipient::new(add)),
//             rt.clone(),
//             Box::new(ActixRecipient::new(ar.recipient())),
//         );

//         Self {
//             src: rt,
//             tx: ar2,
//             rx: rts.start(),
//         }
//     }

//     pub async fn sub(&self, on: ChannelId) -> Result<(), Box<dyn Error>> {
//         self.tx.send(SubMsg { on }).await.map_err(|f| f.into())
//     }

//     pub async fn publish(&self, payload: &str, on: ChannelId) -> Result<(), Box<dyn Error>> {
//         self.tx
//             .send(PubMsg {
//                 to: on,
//                 payload: payload.into(),
//             })
//             .await
//             .map_err(|f| f.into())
//     }

//     pub async fn unsub(&self, on: ChannelId) -> Result<(), Box<dyn Error>> {
//         self.tx.send(UnsubMsg { on }).await.map_err(|f| f.into())
//     }

//     pub fn add_entry(&self, for_service: ServiceId, handler: Arc<PacketHandler>) {
//         self.src.sinks.insert(for_service, handler);
//     }

//     pub fn id(&self) -> ServiceId {
//         self.src.self_id
//     }

//     pub fn create_handler(&self) -> Arc<PacketHandler> {
//         Arc::new(ActixPacketRecipient::new(self.rx.clone().recipient()))
//     }
// }

// pub struct Printer {
//     c: u32,
// }

// impl Printer {
//     pub fn new() -> Self {
//         Printer { c: 0 }
//     }
// }

// impl Actor for Printer {
//     type Context = Context<Self>;
// }

// impl Handler<Payload> for Printer {
//     type Result = ();
//     fn handle(&mut self, msg: Payload, _ctx: &mut Self::Context) -> Self::Result {
//         println!("{} got {:?}", self.c, msg);
//     }
// }
