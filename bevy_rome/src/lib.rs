use bevy::ecs::entity::Entity;
use bevy::math::Vec3;
use bevy::transform::components::Transform;
use serde::{Deserialize, Serialize, Serializer};
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
pub enum Error {
    Unknown,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Unknown // TODO
    }
}

impl From<ron::Error> for Error {
    fn from(err: ron::Error) -> Self {
        Error::Unknown // TODO
    }
}

trait Message<'de, T>: Serialize + Deserialize<'de> + Send {
    fn undo(&mut self, target: &mut T);
    fn redo(&mut self, target: &mut T);
}

#[derive(Serialize, Deserialize)]
struct PosMsg {
    pos: Vec3,
}

impl Message<'_, Transform> for PosMsg {
    fn undo(&mut self, target: &mut Transform) {
        std::mem::swap(&mut target.translation, &mut self.pos);
    }
    fn redo(&mut self, target: &mut Transform) {
        std::mem::swap(&mut target.translation, &mut self.pos);
    }
}

struct SendQueue<W: Write> {
    serializer: ron::ser::Serializer<W>,
}

impl<W: Write> SendQueue<W> {
    pub fn new(writer: W) -> Self {
        let serializer = ron::ser::Serializer::new(writer, None, false).unwrap();
        Self { serializer }
    }

    pub fn send<'de, T>(&mut self, msg: impl Message<'de, T>, target: Entity) -> Result<(), Error> {
        msg.serialize(&mut self.serializer)?;
        Ok(())
    }
}

struct RecvQueue {}

impl RecvQueue {
    pub fn new() -> Self {
        Self {}
    }

    pub fn recv<'de, T, M: Message<'de, T>>(&mut self, bytes: &'de [u8]) -> Result<M, Error> {
        let mut deserializer = ron::de::Deserializer::from_bytes(bytes)?;
        let msg = M::deserialize(&mut deserializer)?;
        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name() -> Result<(), Error> {
        std::thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:34254").unwrap();
            let stream = match listener.accept() {
                Ok((stream, addr)) => {
                    println!("new client: {addr:?}");
                    stream
                }
                Err(e) => {
                    println!("couldn't get client: {e:?}");
                    return;
                }
            };
            let mut queue = RecvQueue::new();
            let buf : [u8; 1024] = [0; 1024];
            let len = stream.read(&mut buf).unwrap();


            TODO - need to decode the message type first!!!!


            let msg = queue.recv(&buf[..len]);
        });

        let stream = TcpStream::connect("127.0.0.1:34254")?;
        let mut queue = SendQueue::new(stream);
        let msg = PosMsg {
            pos: Vec3::splat(1.),
        };
        let target = Entity::from_raw(42);
        queue.send(msg, target)?;
        Ok(())
    }
}
