use std::collections::VecDeque;
use std::ops::Deref;
use std::sync::Mutex;
use std::time;
use tonic::{async_trait, Request, Response, Status, transport::Server};
use crate::simple_queue::{DequeueRequest, DequeueResponse, EnqueueRequest, EnqueueResponse, SizeRequest, SizeResponse};
use crate::simple_queue::queue_server::{Queue, QueueServer};
use tonic_reflection;
use rand::prelude::*;

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
        std::thread::sleep(time::Duration::from_millis(rand::thread_rng().gen_range(1..500)));
        let mut grabbed_lock = self.queue.lock() .unwrap();

        std::thread::sleep(time::Duration::from_millis(rand::thread_rng().gen_range(1..2000)));
        grabbed_lock.push_back(request.get_ref().number);

        std::thread::sleep(time::Duration::from_millis(rand::thread_rng().gen_range(1..500)));
        Ok(Response::new(simple_queue::EnqueueResponse {
            confirmation: { "cool".to_string() }
        }))
    }

    async fn dequeue(&self, request: Request<DequeueRequest>) -> Result<Response<DequeueResponse>, Status> {
        let num_to_pop = request.get_ref().number;
        let mut return_vec = Vec::new();
        let mut q = self.queue.lock().unwrap();
        for n in 0..num_to_pop {
            match q.pop_front() {
                Some(n) => return_vec.push(n),
                None => continue,
            }
        }

        Ok(Response::new(simple_queue::DequeueResponse {
            numbers : { return_vec }
        }))

    }

    async fn size(&self, request: Request<SizeRequest>) -> Result<Response<SizeResponse>, Status> {
        let s = self.queue.lock().unwrap().len() as i32;

        Ok(Response::new(SizeResponse {
            size: { s }
        }))
    }
}