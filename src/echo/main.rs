use std::sync::Arc;
use maelstrom::{done, Node, Result, Runtime};
use maelstrom::protocol::Message;
use tonic::async_trait;

pub(crate) fn main() -> Result<()> {
    Runtime::init(try_main())
}

async fn try_main() -> Result<()> {
    let handler = Arc::new(EchoHandler::default());
    Runtime::new().with_handler(handler).run().await
}

#[derive(Clone, Default, Debug)]
struct EchoHandler {}

#[async_trait]
impl Node for EchoHandler {
    async fn process(&self, runtime: Runtime, request: Message) -> Result<()> {
        if request.get_type() == "echo" {
            let echo = request.body.clone().with_type("echo_ok");
            return runtime.reply(request, echo).await;
        }

        done(runtime, request)
    }
}

