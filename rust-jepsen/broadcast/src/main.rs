//! See https://fly.io/dist-sys/3a/

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use maelstrom::{done, Node, Result, Runtime};
use maelstrom::protocol::{Message, MessageBody};

pub fn main() -> Result<()> {
    Runtime::init(try_main())
}

async fn try_main() -> Result<()> {
    Runtime::new().with_handler(Arc::new(BroadcastHandler::new())).run().await
}

struct BroadcastHandler {
    msgs: Arc<RwLock<Vec<i64>>>,
}

impl BroadcastHandler {
    const BROADCAST_MSG: &'static str = "broadcast";
    const BROADCAST_MSG_OK: &'static str = "broadcast_ok";
    const READ_MSG: &'static str = "read";
    const READ_MSG_OK: &'static str = "read_ok";
    const TOPOLOGY_MSG: &'static str = "topology";
    const TOPOLOGY_MSG_OK: &'static str = "topology_ok";

    // fn new(msgs: Arc<RwLock<Vec<i64>>>) -> Self {
    fn new() -> Self {
        BroadcastHandler {
            msgs: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Node for BroadcastHandler {
    async fn process(&self, runtime: Runtime, request: Message) -> Result<()> {
        let res = match request.body.typ.as_str() {
            Self::BROADCAST_MSG => {
                // 1. save new message value
                {
                    let v = request.body.extra["message"].as_i64().expect("expected an integer");
                    let mut guard = self.msgs.write().expect("lock is poisoned");
                    guard.push(v);
                }

                // 2. broadcast to all nodes
                runtime.nodes().iter()
                    .filter(|n| *n != runtime.node_id())
                    .for_each(|node| {
                        let body = request.body.clone();
                        println!("forwarding {:?} to {node}", body);
                        runtime.send_async(node, &body)
                            .expect("send failure to {node}: {request}");
                    });

                // 3. ack message
                let resp = MessageBody::new().with_type(Self::BROADCAST_MSG_OK);
                Ok(runtime.reply(request.clone(), resp).await?)
            }
            Self::READ_MSG => {
                let body = if let Ok(guard) = self.msgs.read() {
                    let mut resp = request.body.clone().with_type(Self::READ_MSG_OK);
                    let msgs = serde_json::to_value(&*guard).unwrap();
                    resp.extra.insert(String::from("messages"), msgs);
                    resp
                } else {
                    panic!("lock poisoned");
                };

                Ok(runtime.reply(request.clone(), body).await?)
            }
            Self::TOPOLOGY_MSG => {
                // for now don't need to do anything
                let resp = MessageBody::new().with_type(Self::TOPOLOGY_MSG_OK);
                Ok(runtime.reply(request.clone(), resp).await?)
            }
            _ => Ok(())
        };

        res.map_err(|e: Box<dyn std::error::Error + Send + Sync>| {
            eprintln!("{e}");
            done(runtime, request)
        }).or(Ok(()))
    }
}
