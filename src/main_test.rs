use std::{thread::JoinHandle, time::Instant};

use r_table::RoutingTable;
use reciever::RouterRx;
use tokio::{
    runtime,
    sync::{mpsc, oneshot},
};

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

    let count = 10_000;

    let wi = ChannelId::get_random();

    let (r1, _) = Router::new_ping_pong_test(1, wi, count - 1);

    let r2 = Router::new_test(2);

    let r3 = Router::new_test(3);

    // let r4 = Router::new_test(4);

    let r5 = Router::new_test(5);

    let (r6, rec) = Router::new_ping_pong_test(6, wi, count);

    //2
    // r1.connect_to(&r6);

    //3
    // r1.connect_to(&r3);
    // r3.connect_to(&r6);

    //4
    // r1.connect_to(&r2);
    // r2.connect_to(&r3);
    // r3.connect_to(&r6);

    //5
    r1.connect_to(&r2);
    r2.connect_to(&r3);
    r3.connect_to(&r5);
    r5.connect_to(&r6);

    //6
    // r1.connect_to(&r2);
    // r2.connect_to(&r3);
    // r3.connect_to(&r4);
    // r4.connect_to(&r5);
    // r5.connect_to(&r6);

    // r1.connect_to(&r2);

    // r2.connect_to(&r3);

    // r3.connect_to(&r4);

    // r2.connect_to(&r5);

    // r3.connect_to(&r6);

    r1.sender.send_sub_msg(wi);

    r6.sender.send_sub_msg(wi);

    let ins = Instant::now();

    r1.sender.send_pub_msg(wi, "ping").await;

    let _ = rec.await;

    println!("{:?}", Instant::now() - ins)

    //now we wait for channel to complete

    //setup ping pong test

    // rou2.send();

    //test with 2 router instances and see if messages message or not
}

// fn test_main_multi() {
//     pretty_env_logger::init();

//     println!("Hello World");

//     let count = 1000_000;

//     let wi = ChannelId::get_random();

//     let (r1, rec1, h1) = setup_router_sng_thread(1, Some(wi), count - 1);

//     let (r2, _, h2) = setup_router_sng_thread(2, None, 0);

//     let (r3, _, h3) = setup_router_sng_thread(3, None, 0);

//     let (r4, _, h4) = setup_router_sng_thread(4, None, 0);

//     let (r5, _, h5) = setup_router_sng_thread(5, None, 0);

//     let (r6, rec, h6) = setup_router_sng_thread(6, Some(wi), count);

//     //2
//     // r1.connect_to(&r6);

//     //3
//     // r1.connect_to(&r3);
//     // r3.connect_to(&r6);

//     //4
//     // r1.connect_to(&r2);
//     // r2.connect_to(&r3);
//     // r3.connect_to(&r6);

//     //6
//     r1.connect_to(&r2);
//     r2.connect_to(&r3);
//     r3.connect_to(&r4);
//     r4.connect_to(&r5);
//     r5.connect_to(&r6);

//     // r1.connect_to(&r2);

//     // r2.connect_to(&r3);

//     // r3.connect_to(&r4);

//     // r2.connect_to(&r5);

//     // r3.connect_to(&r6);

//     r1.sender.send_sub_msg(wi);

//     r6.sender.send_sub_msg(wi);

//     let _ = h1.join().unwrap();
//     let _ = h2.join().unwrap();
//     let _ = h3.join().unwrap();
//     let _ = h4.join().unwrap();
//     let _ = h5.join().unwrap();

//     let ins = Instant::now();

//     let runtime = runtime::Builder::new_multi_thread()
//             .worker_threads(8)
//             .enable_all()
//             .build()
//             .unwrap();

//     runtime.block_on(async {
//         r1.sender.send_pub_msg(wi, "ping").await;

//         let c2 = rec1.await.unwrap();
//         let c = rec.await.unwrap();

//         println!("{:?} {c2} {c}", Instant::now() - ins)
//     });

//     //now we wait for channel to complete

//     //setup ping pong test

//     // rou2.send();

//     //test with 2 router instances and see if messages message or not
// }

// fn setup_router_sng_thread(
//     c: u32,
//     wi: Option<ChannelId>,
//     lim: u32,
// ) -> (Router, oneshot::Receiver<u32>, JoinHandle<()>) {
//     let (tx, mut rx) = mpsc::unbounded_channel();
//     let (donetx, donerx) = oneshot::channel();
//     let (mut rr, ptx) = RouterRx::new(c, RoutingTable::default(), tx);
//     let rtx = rr.create_tx();
//     let localrtx = rtx.clone();

//     let handle = std::thread::spawn(move || {
//         let runtime = runtime::Builder::new_multi_thread()
//             .worker_threads(8)
//             .enable_all()
//             .build()
//             .unwrap();

//         runtime.block_on(async {
//             tokio::spawn(async move {
//                 rr.recv_packets().await;
//             });

//             if let Some(wi) = wi {
//                 tokio::spawn(async move {
//                     let mut i = 0;
//                     while let Some(t) = rx.recv().await {
//                         if t == "ping" {
//                             localrtx.send_pub_msg(wi, "pong").await;
//                         } else if t == "pong" {
//                             localrtx.send_pub_msg(wi, "ping").await;
//                         }
//                         if i == lim {
//                             let _ = donetx.send(i);
//                             break;
//                         }
//                         i += 1;
//                     }
//                 });
//             }
//         });
//     });

//     (Router { ptx, sender: rtx }, donerx, handle)
// }
