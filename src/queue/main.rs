mod client;

use nanoid::nanoid;
use prost::Message;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::pin::Pin;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use tonic::codegen::tokio_stream::Stream;
use tonic::service::Interceptor;
use tonic::{async_trait, transport::Server, Request, Response, Status, Streaming};
use zeyrho::queue::wal::wal::{FileWal, Wal};
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

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let queue = Arc::new(Mutex::new(VecDeque::new()));

    let wal = FileWal::new("data/wal.wal", "data/wal.meta").unwrap();

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

struct SimpleQueue {
    queue: Arc<Mutex<VecDeque<i32>>>,
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
    async fn enqueue(
        &self,
        request: Request<EnqueueRequest>,
    ) -> Result<Response<EnqueueResponse>, Status> {
        let mut buf = Vec::new();
        let number = request.get_ref().number;
        request.into_inner().encode(&mut buf).unwrap();
        self.wal.lock().unwrap().write(&buf)?;

        self.queue.lock().unwrap().push_back(number);

        Ok(Response::new(EnqueueResponse {
            confirmation: { "".to_string() },
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

        Ok(Response::new(DequeueResponse {
            numbers: { return_vec },
        }))
    }

    async fn size(&self, request: Request<SizeRequest>) -> Result<Response<SizeResponse>, Status> {
        let s = self.queue.lock().unwrap().len() as i32;

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
