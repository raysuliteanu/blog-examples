//! See https://fly.io/dist-sys/1/
use async_trait::async_trait;
use maelstrom::protocol::Message;
use maelstrom::{done, Node, Result, Runtime};
use std::sync::Arc;

pub fn main() -> Result<()> {
    Runtime::init(try_main())
}

async fn try_main() -> Result<()> {
    let handler = Arc::new(EchoHandler::default());
    Runtime::new().with_handler(handler).run().await
}

#[derive(Clone, Default)]
struct EchoHandler {}

const ECHO_MSG_TYPE: &str = "echo";
const ECHO_REPLY_MSG_TYPE: &str = "echo_ok";

#[async_trait]
impl Node for EchoHandler {
    async fn process(&self, runtime: Runtime, request: Message) -> Result<()> {
        let message_body = &request.body;
        if message_body.typ == ECHO_MSG_TYPE
            && runtime
                .reply(
                    request.clone(),
                    message_body.clone().with_type(ECHO_REPLY_MSG_TYPE),
                )
                .await
                .is_ok()
        {
            return Ok(());
        }

        done(runtime, request)
    }
}
