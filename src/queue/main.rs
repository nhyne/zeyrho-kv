mod client;

use bytes::Bytes;
use nanoid::nanoid;
use prost::Message;
use std::collections::VecDeque;
use std::io::Write;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tonic::codegen::tokio_stream::Stream;
use tonic::service::Interceptor;
use tonic::{async_trait, transport::Server, Request, Response, Status, Streaming};
use tracing::{info, instrument};
use zeyrho::queue::wal::wal::{FileWal, Wal};
use zeyrho::zeyrho::queue::dequeue_response::QueueMessage;
use zeyrho::zeyrho::queue::queue_server::{Queue, QueueServer};
use zeyrho::zeyrho::queue::{
    DequeueRequest, DequeueResponse, EnqueueRequest, EnqueueResponse, ReplicateDataRequest,
    ReplicateDataResponse, SizeRequest, SizeResponse,
};

const DATA_DIR: &str = "data";

mod proto {
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("queue_descriptor");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = "127.0.0.1:8080".parse().unwrap();

    tracing_subscriber::fmt::init();

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let queue = Arc::new(Mutex::new(VecDeque::new()));

    let wal = FileWal::new("data/wal.bin", "data/wal.meta").unwrap();

    let queue_service = SimpleQueue {
        queue,
        wal: Arc::new(Mutex::new(wal)),
    };

    Server::builder()
        .add_service(service)
        .add_service(QueueServer::with_interceptor(
            queue_service,
            LoadShed {
                shed: Arc::new(Mutex::new(false)),
            },
        ))
        .serve(address)
        .await?;

    Ok(())
}

#[derive(Debug)]
struct SimpleQueue {
    queue: Arc<Mutex<VecDeque<QueueMessage>>>,
    wal: Arc<Mutex<FileWal>>,
}

#[derive(Debug, Default, Clone)]
struct LoadShed {
    // would want to do some sort of "bucketing" or "how much load do we have?"
    // and then flip this flag, and we'll start rejecting requests
    // Idea is explained in this blog post: https://www.warpstream.com/blog/dealing-with-rejection-in-distributed-systems
    shed: Arc<Mutex<bool>>,
}

impl Interceptor for LoadShed {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let grabbed_lock = self.shed.lock().unwrap();

        let current_val = *grabbed_lock;

        if current_val {
            Err(Status::resource_exhausted("too many requests"))
        } else {
            Ok(request)
        }
    }
}

#[async_trait]
impl Queue for SimpleQueue {
    #[instrument]
    async fn enqueue(
        &self,
        request: Request<EnqueueRequest>,
    ) -> Result<Response<EnqueueResponse>, Status> {
        let mut buf = Vec::new();
        let message_id = nanoid!();
        let queue_message = QueueMessage {
            id: message_id.clone(),
            payload: request.get_ref().payload.clone(),
        };
        self.queue.lock().unwrap().push_back(queue_message);
        request.into_inner().encode(&mut buf).unwrap();
        self.wal.lock().unwrap().write(&buf)?;

        Ok(Response::new(EnqueueResponse {
            message_id: { message_id },
        }))
    }

    async fn dequeue(
        &self,
        request: Request<DequeueRequest>,
    ) -> Result<Response<DequeueResponse>, Status> {
        let num_to_pop = request.get_ref().number;
        let mut return_vec = Vec::new();
        let mut q = self.queue.lock().unwrap();
        for _ in 0..num_to_pop {
            match q.pop_front() {
                Some(n) => return_vec.push(n),
                None => continue,
            }
        }

        let response = DequeueResponse {
            messages: return_vec,
        };

        Ok(Response::new(response))
    }

    async fn size(&self, request: Request<SizeRequest>) -> Result<Response<SizeResponse>, Status> {
        let s = self.queue.lock().unwrap().len() as u64;

        Ok(Response::new(SizeResponse { size: { s } }))
    }

    type ReplicateDataStream =
        Pin<Box<dyn Stream<Item = Result<ReplicateDataResponse, Status>> + Send>>;

    async fn replicate_data(
        &self,
        request: Request<Streaming<ReplicateDataRequest>>,
    ) -> Result<Response<Self::ReplicateDataStream>, Status> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_decode() {
        let data = b"\x08\x01";
        let bytes = Bytes::from(data.to_vec());
        let request = EnqueueRequest::decode(bytes).unwrap();
        assert_eq!(request.payload, b"1");
    }
}
