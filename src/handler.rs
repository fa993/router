use std::{error::Error, fmt::Debug};

use actix::Recipient;

use crate::r_table::Packet;

pub trait MessageHandler<T>: Debug + Sync + Send {
    fn try_handle(&self, p: T) -> Result<(), Box<dyn Error>>;
}

pub type PacketHandler = dyn MessageHandler<Packet>;

#[derive(Debug)]
pub struct ActixRecipient<T: actix::Message + Send + Sync + Debug>
where
    <T as actix::Message>::Result: Send,
{
    inner: Recipient<T>,
}

impl<T: actix::Message + Send + Sync + Debug + 'static> MessageHandler<T> for ActixRecipient<T>
where
    <T as actix::Message>::Result: Send,
{
    fn try_handle(&self, p: T) -> Result<(), Box<dyn Error>> {
        self.inner.try_send(p).map_err(|f| f.into())
    }
}

impl<T: actix::Message + Send + Sync + Debug> ActixRecipient<T>
where
    <T as actix::Message>::Result: Send,
{
    pub fn new(r: Recipient<T>) -> ActixRecipient<T> {
        ActixRecipient { inner: r }
    }
}

pub type ActixPacketRecipient = ActixRecipient<Packet>;
