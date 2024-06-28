//! See https://fly.io/dist-sys/2/

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use maelstrom::{done, Node, Result, Runtime};
use maelstrom::protocol::Message;

pub fn main() -> Result<()> {
    Runtime::init(try_main())
}

async fn try_main() -> Result<()> {
    Runtime::new().with_handler(Arc::new(UniqueIdHandler::new())).run().await
}

struct UniqueIdHandler {
    next_id: AtomicU64,
}

impl UniqueIdHandler {
    fn new() -> Self {
        UniqueIdHandler {
            next_id: AtomicU64::new(0)
        }
    }
}

trait IdGenerator {
    fn next_id(&self, prefix: &str) -> String;
}

impl IdGenerator for UniqueIdHandler {
    fn next_id(&self, x: &str) -> String {
        let id = self.next_id.fetch_add(1, Ordering::AcqRel);
        format!("{}{}", x, id)
    }
}

const GEN_ID_MSG_TYPE: &str = "generate";
const GEN_ID_REPLY_MSG_TYPE: &str = "generate_ok";

#[async_trait]
impl Node for UniqueIdHandler {
    async fn process(&self, runtime: Runtime, request: Message) -> Result<()> {
        let resp = &mut request.body.clone().with_type(GEN_ID_REPLY_MSG_TYPE);
        let next_id_value = self.next_id(runtime.node_id());
        resp.extra.insert("id".to_string(), next_id_value.into());

        let message_body = &request.body;
        if message_body.typ == GEN_ID_MSG_TYPE && runtime.reply(request.clone(), resp).await.is_ok()
        {
            return Ok(());
        }

        done(runtime, request)
    }
}
