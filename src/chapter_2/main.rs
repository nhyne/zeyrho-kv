mod client;

use nanoid::nanoid;
use rand::prelude::*;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use std::{fs, time};
use std::sync::mpsc::{channel, Receiver};
use std::thread::spawn;
use tonic::service::Interceptor;
use tonic::{async_trait, transport::Server, Request, Response, Status};
use tonic_reflection;
use zeyrho::zeyrho::kv_store::kv_store_server::{KvStore, KvStoreServer};
use zeyrho::zeyrho::kv_store::{
    GetResponse, GetRequest, SetRequest, SetResponse, DeleteRequest, DeleteResponse,
};

const DATA_DIR: &str = "data";

mod proto {
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("kv_store_descriptor");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = "127.0.0.1:8080".parse().unwrap();

    let (sender, receiver) = channel();

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let queue = Arc::new(Mutex::new(HashMap::new()));
    let cloned_queue = queue.clone();
    let queue_service = SimpleKvStore {
        hash_map: queue,
        sender,
    };

    let handler = spawn(move || {
        for journaled in receiver {
            // TODO: We should not be cloning the queue mutex every time we process a message, there should just be one owned queue mutex in this thread
            let mut grabbed_lock = cloned_queue.lock().unwrap();

            // I don't believe that this is good rust code, but I'm not sure how else to just pass the contents of the lock...
            let journal_result = process_journal_file(journaled, &mut *grabbed_lock);
            match journal_result {
                Ok(_) => continue,
                Err(e) => println!("error processing journal: {}", e),
            }
        }
    });

    Server::builder()
        .add_service(service)
        .add_service(KvStoreServer::with_interceptor(
            queue_service,
            LoadShed {
                shed: Arc::new(Mutex::new(false)),
            },
        ))
        .serve(address)
        .await?;

    Ok(())
}

struct SimpleKvStore {
    hash_map: Arc<Mutex<HashMap<String, i32>>>,
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
        let mut grabbed_lock = self.shed.lock().unwrap();

        let current_val = grabbed_lock.clone();

        if current_val {
            Err(Status::resource_exhausted("too many requests"))
        } else {
            Ok(request)
        }
    }
}

fn journal_request(request: &SetRequest) -> Result<String, std::io::Error> {
    let id = nanoid!();

    let mut buf = Vec::new();
    request.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut file = File::create(&("data/".to_string() + &id)).expect("creating file failed");
    file.write_all(&buf)?;
    file.flush()?;

    Ok(id)
}

fn process_journal_file(file_name: String, hash_map: &mut HashMap<String, i32>) -> Result<(), std::io::Error> {
    let mut buf = Vec::new();
    let mut file = File::open(&("data/".to_string() + &file_name)).expect("opening file failed");
    file.read_to_end(&mut buf)?;

    let byte_slice: &[u8] = &buf;

    let mut de = Deserializer::new(byte_slice);
    let request_body: SetRequest = Deserialize::deserialize(&mut de).unwrap();

    let key_name = request_body.key.clone();
    hash_map.insert(request_body.key, request_body.value);

    fs::remove_file(&("data/".to_string() + &file_name))?;
    println!("key was: {}, value was: {}", key_name, request_body.value);
    Ok(())
}

#[async_trait]
impl KvStore for SimpleKvStore {
    async fn set(&self, request: Request<SetRequest>) -> Result<Response<SetResponse>, Status> {
        let journal_id = journal_request(request.get_ref())
            .map_err(|_| Status::internal("error journaling request"))?;

        self.sender.send(journal_id).map_err(|e| Status::internal("error queueing journal for processing"))?;

        Ok(Response::new(SetResponse {
            confirmation: true,
        }))
    }

    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let mut q = self.hash_map.lock().unwrap();

        let val = q.get(&request.get_ref().key);

        Ok(Response::new(GetResponse {
            value: val.copied(),
        }))
    }

    async fn delete(&self, request: Request<DeleteRequest>) -> Result<Response<DeleteResponse>, Status> {
        let mut q = self.hash_map.lock().unwrap();

        let val = q.remove(&request.get_ref().key);

        Ok(Response::new(DeleteResponse {
            confirmation: match val {
                None => false,
                Some(_) => true,
            }
        }))
    }
}
