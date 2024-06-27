//! See https://fly.io/dist-sys/2/

use std::mem::MaybeUninit;
use async_trait::async_trait;
use maelstrom::protocol::Message;
use maelstrom::{done, Node, Result, Runtime};
use std::str::FromStr;
use std::sync::{Arc, Mutex, Once};

pub fn main() -> Result<()> {
    Runtime::init(try_main())
}

async fn try_main() -> Result<()> {
    let runtime = Runtime::new();
    let generator_handler = UniqueIdHandler::new();
    let handler = Arc::new(generator_handler);
    runtime.with_handler(handler).run().await
}

fn extract_node_number(runtime: &Runtime) -> u8 {
    u8::from_str(runtime.node_id().strip_prefix('n').unwrap()).unwrap()
}

#[derive(Clone)]
struct UniqueIdHandler {
    next_id: Arc<Mutex<MaybeUninit<u64>>>,
}

impl UniqueIdHandler {
    fn new() -> Self {
        // let next = (node_num as u64) << (64 - std::mem::size_of::<u8>());
        UniqueIdHandler {
            next_id: Arc::new(Mutex::new(MaybeUninit::uninit())),
        }
    }
}

trait IdGenerator {
    fn next_id(&self) -> u64;
}

impl IdGenerator for UniqueIdHandler {
    fn next_id(&self) -> u64 {
        let mut id = self.next_id.lock().unwrap();
        let current = unsafe { id.assume_init() };
        id.write(current + 1);
        current
    }
}

const GEN_ID_MSG_TYPE: &str = "generate";
const GEN_ID_REPLY_MSG_TYPE: &str = "generate_ok";

static INIT: Once = Once::new();

#[async_trait]
impl Node for UniqueIdHandler {
    async fn process(&self, runtime: Runtime, request: Message) -> Result<()> {
        if !INIT.is_completed() {
            eprintln!("initializing node");
            INIT.call_once(|| {
                let node = extract_node_number(&runtime);
                let start = (node as u64) << (64 - std::mem::size_of::<u8>());
                let mut start_id = self.next_id.lock().unwrap();
                start_id.write(start);
                eprintln!("initializing start index to {start}");
            });
        }
        
        let resp = &mut request.body.clone().with_type(GEN_ID_REPLY_MSG_TYPE);
        resp.extra.insert("id".to_string(), Into::into(self.next_id()));

        let message_body = &request.body;
        if message_body.typ == GEN_ID_MSG_TYPE && runtime.reply(request.clone(), resp).await.is_ok()
        {
            return Ok(());
        }

        done(runtime, request)
    }
}
