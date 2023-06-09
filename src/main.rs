use std::time::Duration;

use tokio::time::interval;

use crate::{
    r_table::{ChannelId, Randomable},
    router::Router,
};

pub mod r_table;
pub mod reciever;
pub mod router;
pub mod transmitter;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    println!("Hello World");

    // let rt = RoutingTable::default();

    // let (tx, rx) = mpsc::unbounded_channel();

    // let (mut rr, ptx) = RouterRx::new(ServiceId::get_random(), rt, tx);

    // let ro = rr.create_tx();
    // let r1 = ro.clone();
    // tokio::spawn(async move {
    //     rr.recv_packets().await;
    // });

    // tokio::spawn(async move {
    //     r1.handle_msg(reciever::OutgoingMessage::Pub(
    //         "Hello World".to_string(),
    //         ChannelId::get_random(),
    //     ))
    //     .await;
    // });

    // tokio::spawn(async move {
    //     ro.handle_msg(reciever::OutgoingMessage::Pub(
    //         "Hello World".to_string(),
    //         ChannelId::get_random(),
    //     ))
    //     .await;
    // });

    let r1 = Router::new_test(1);

    let r2 = Router::new_test(2);

    r1.connect_to(&r2);

    let wi = ChannelId::get_random();

    r1.sender.send_sub_msg(wi);

    r2.sender.send_sub_msg(wi);

    let handle = tokio::spawn(async move {
        r1.sender.send_pub_msg(wi, "Hello World").await;
    });

    let _ = handle.await;

    let mut inter = interval(Duration::from_secs(2));

    inter.tick().await;

    r2.sender.send_pub_msg(wi, "Hello world").await;

    let mut inter = interval(Duration::from_secs(2));

    inter.tick().await;

    // r1.sender.send_pub_msg(wi, "Hello World").await;

    // tokio::spawn(async {
    //     ro.send_pub_msg(Uuid::new_v4(), "Hello world");
    // });

    // tokio::spawn(async {
    //     ro.send_pub_msg(Uuid::new_v4(), "Hello world");
    // });

    // let r1 = Router::new_test(1);

    // let r2 = Router::new_test(2);

    // let r3 = Router::new_test(3);

    // let r4 = Router::new_test(4);

    // let r5 = Router::new_test(5);

    // let r6 = Router::new_test(6);

    // r1.add_entry(r2.id(), r2.create_handler());
    // r2.add_entry(r1.id(), r1.create_handler());

    // r5.add_entry(r2.id(), r2.create_handler());
    // r2.add_entry(r5.id(), r5.create_handler());

    // r2.add_entry(r3.id(), r3.create_handler());
    // r3.add_entry(r2.id(), r2.create_handler());

    // r6.add_entry(r3.id(), r3.create_handler());
    // r3.add_entry(r6.id(), r6.create_handler());

    // r3.add_entry(r4.id(), r4.create_handler());
    // r4.add_entry(r3.id(), r3.create_handler());

    // let wi = Uuid::new_v4();

    // r1.sub(wi).await.unwrap();

    // r3.sub(wi).await.unwrap();

    // r1.publish("Hello World", wi).await.unwrap();

    // r5.sub(wi).await.unwrap();

    // r3.publish("Hello to you too good sir", wi).await.unwrap();

    // r5.publish("Yolo", wi).await.unwrap();

    // r3.unsub(wi).await.unwrap();

    // r5.publish("Yolo2", wi).await.unwrap();

    // r3.sub(wi).await.unwrap();

    // r5.unsub(wi).await.unwrap();

    // r1.publish("Hehe", wi).await.unwrap();

    // rou2.send();

    //test with 2 router instances and see if messages message or not
}
