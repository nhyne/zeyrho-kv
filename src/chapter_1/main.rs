use std::collections::VecDeque;
use std::ops::Deref;
use std::sync::Mutex;
use tonic::{async_trait, Request, Response, Status, transport::Server};
use crate::simple_queue::{DequeueRequest, DequeueResponse, EnqueueRequest, EnqueueResponse, SizeRequest, SizeResponse};
use crate::simple_queue::queue_server::{Queue, QueueServer};
use tonic_reflection;

mod simple_queue;

mod proto {
    tonic::include_proto!("simple_queue");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("simple_queue_descriptor");
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = "127.0.0.1:8080".parse().unwrap();

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let queue_service = SimpleQueue{queue: Mutex::new(VecDeque::new())};

    Server::builder()
        .add_service(service)
        .add_service(QueueServer::new(queue_service))
        .serve(address)
        .await?;
    Ok(())
}

struct SimpleQueue {
    queue: Mutex<VecDeque<i32>>
}


#[async_trait]
impl Queue for SimpleQueue {
    async fn enqueue(&self, request: Request<EnqueueRequest>) -> Result<Response<EnqueueResponse>, Status> {
        let mut q = self.queue.lock().unwrap();
        q.push_back(request.into_inner().number);

        Ok(Response::new(simple_queue::EnqueueResponse {
            confirmation: { "cool".to_string() }
        }))
    }

    async fn dequeue(&self, request: Request<DequeueRequest>) -> Result<Response<DequeueResponse>, Status> {
        todo!()
    }

    async fn size(&self, request: Request<SizeRequest>) -> Result<Response<SizeResponse>, Status> {
        let g = self.queue.lock().unwrap();
        let s = g.len() as i32;

        println!("{}", s);
        Ok(Response::new(SizeResponse {
            size: { s }
        }))
    }
}