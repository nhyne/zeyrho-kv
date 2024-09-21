use std::collections::VecDeque;
use std::error::Error;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time;
use tonic::{async_trait, Request, Response, Status, transport::Server};
use zeyrho::simple_queue::simple_queue::{DequeueRequest, DequeueResponse, EnqueueRequest, EnqueueResponse, SizeRequest, SizeResponse};
use zeyrho::simple_queue::simple_queue::queue_server::{Queue, QueueServer};
use tonic_reflection;
use rand::prelude::*;
use tonic::service::Interceptor;
use nanoid::nanoid;


const DATA_DIR: &str = "data";

mod proto {
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
        .add_service(QueueServer::with_interceptor(queue_service, LoadShed{shed: Arc::new(Mutex::new(true))}))
        .serve(address)
        .await?;
    Ok(())
}

struct SimpleQueue {
    queue: Mutex<VecDeque<i32>>
}


#[derive(Debug, Default, Clone)]
struct LoadShed {
    shed: Arc<Mutex<bool>>,
}

impl Interceptor for LoadShed {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let mut grabbed_lock = self.shed.lock().unwrap();

        let current_val = grabbed_lock.clone();
        *grabbed_lock = !*grabbed_lock;

        if current_val {
            Err(Status::resource_exhausted("too many requests"))
        } else {
            Ok(request)
        }
    }
}


// fn intercept(mut req: Request<()>) -> Result<Request<()>, Status> {
//     println!("Intercepting request: {:?}", req);
//
//     Err(Status::resource_exhausted("too many requests"))
//
//     // Ok(req)
// }

#[async_trait]
impl Queue for SimpleQueue {
    async fn enqueue(&self, request: Request<EnqueueRequest>) -> Result<Response<EnqueueResponse>, Status> {
        // TODO: Add jounaling here before writing to our queue

        std::thread::sleep(time::Duration::from_millis(rand::thread_rng().gen_range(1..500)));
        let mut grabbed_lock = self.queue.lock() .unwrap();

        std::thread::sleep(time::Duration::from_millis(rand::thread_rng().gen_range(1..2000)));
        grabbed_lock.push_back(request.get_ref().number);

        std::thread::sleep(time::Duration::from_millis(rand::thread_rng().gen_range(1..500)));
        Ok(Response::new(EnqueueResponse {
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

        Ok(Response::new(DequeueResponse {
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
