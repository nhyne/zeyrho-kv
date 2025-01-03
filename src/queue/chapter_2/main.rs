mod client;

use nanoid::nanoid;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::pin::Pin;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use tonic::codegen::tokio_stream::Stream;
use tonic::service::Interceptor;
use tonic::{async_trait, transport::Server, Request, Response, Status, Streaming};
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

    let (sender, receiver) = channel();

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let cloned_queue = queue.clone();
    let queue_service = SimpleQueue { queue, sender };

    let handler = spawn(move || {
        for journaled in receiver {
            // TODO: We should not be cloning the queue mutex every time we process a message, there should just be one owned queue mutex in this thread
            let mut grabbed_lock = cloned_queue.lock().unwrap();

            // I don't believe that this is good rust code but I'm not sure how else to just pass the contents of the lock...
            let journal_result = process_journal_file(journaled, &mut grabbed_lock);
            match journal_result {
                Ok(_) => continue,
                Err(e) => println!("error processing journal: {}", e),
            }
        }
    });

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
    sender: std::sync::mpsc::Sender<String>,
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

fn journal_request(request: &EnqueueRequest) -> Result<String, std::io::Error> {
    let id = nanoid!();

    let mut buf = Vec::new();
    request.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut file = File::create(&("data/".to_string() + &id)).expect("creating file failed");
    file.write_all(&buf)?;
    file.flush()?;

    Ok(id)
}

fn process_journal_file(
    file_name: String,
    queue: &mut VecDeque<i32>,
) -> Result<(), std::io::Error> {
    let mut buf = Vec::new();
    let mut file = File::open(&("data/".to_string() + &file_name)).expect("opening file failed");
    file.read_to_end(&mut buf)?;

    let byte_slice: &[u8] = &buf;

    let mut de = Deserializer::new(byte_slice);
    let request_body: EnqueueRequest = Deserialize::deserialize(&mut de).unwrap();

    queue.push_back(request_body.number);

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

        let cloned_id = journal_id.clone();
        self.sender
            .send(journal_id)
            .map_err(|e| Status::internal("error queueing journal for processing"))?;

        Ok(Response::new(EnqueueResponse {
            confirmation: { cloned_id },
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
