use nanoid::nanoid;
use rand::prelude::*;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::{fs, time};
use tonic::service::Interceptor;
use tonic::{async_trait, transport::Server, Request, Response, Status};
use tonic_reflection;
use zeyrho::simple_queue::simple_queue::queue_server::{Queue, QueueServer};
use zeyrho::simple_queue::simple_queue::{
    DequeueRequest, DequeueResponse, EnqueueRequest, EnqueueResponse, SizeRequest, SizeResponse,
};

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

    let queue_service = SimpleQueue {
        queue: Mutex::new(VecDeque::new()),
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
    queue: Mutex<VecDeque<i32>>,
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
        let mut grabbed_lock = self.shed.lock().unwrap();

        let current_val = grabbed_lock.clone();

        if current_val {
            Err(Status::resource_exhausted("too many requests"))
        } else {
            Ok(request)
        }
    }
}

fn journal_request(request: &EnqueueRequest) -> Result<String, std::io::Error> {
    let id = nanoid!();

    let mut buf = Vec::new();
    request.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut file = File::create(&("data/".to_string() + &id)).expect("creating file failed");
    file.write_all(&buf)?;
    file.flush()?;

    Ok(id)
}

fn process_journal_file(file_name: String) -> Result<(), std::io::Error> {

    let mut buf = Vec::new();
    let mut file = File::open(&("data/".to_string() + &file_name)).expect("opening file failed");
    file.read_to_end(&mut buf)?;

    let byte_slice : &[u8] = &buf;

    let mut de = Deserializer::new(byte_slice);
    let request_body : EnqueueRequest = Deserialize::deserialize(&mut de).unwrap();

    fs::remove_file(&("data/".to_string() + &file_name))?;
    println!("number was: {}", request_body.number);
    Ok(())

}

#[async_trait]
impl Queue for SimpleQueue {
    async fn enqueue(
        &self,
        request: Request<EnqueueRequest>,
    ) -> Result<Response<EnqueueResponse>, Status> {
        // TODO: Add jounaling here before writing to our queue
        let journal_id = journal_request(request.get_ref())
            .map_err(|_| Status::internal("error journaling request"))?;

        std::thread::sleep(time::Duration::from_millis(
            rand::thread_rng().gen_range(1..500),
        ));
        let mut grabbed_lock = self.queue.lock().unwrap();

        std::thread::sleep(time::Duration::from_millis(
            rand::thread_rng().gen_range(1..2000),
        ));
        grabbed_lock.push_back(request.get_ref().number);

        std::thread::sleep(time::Duration::from_millis(
            rand::thread_rng().gen_range(1..500),
        ));

        process_journal_file(journal_id)?;

        Ok(Response::new(EnqueueResponse {
            confirmation: { "cool".to_string() },
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
}
