use rand::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::sync::Mutex;
use std::time;
use tonic::{async_trait, transport::Server, Request, Response, Status};
use tonic_reflection;
use zeyrho::zeyrho::kv_store::kv_store_server::{KvStore, KvStoreServer};
use zeyrho::zeyrho::kv_store::{DeleteRequest, DeleteResponse, GetRequest, GetResponse, SetRequest, SetResponse};

mod proto {
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("kv_store_descriptor");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = "127.0.0.1:8080".parse().unwrap();

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let kv_store_service = SimpleKVStore {
        dictionary: Mutex::new(HashMap::new()),
    };

    Server::builder()
        .add_service(service)
        .add_service(KvStoreServer::new(kv_store_service))
        .serve(address)
        .await?;
    Ok(())
}

struct SimpleKVStore {
    dictionary: Mutex<HashMap<String, i32>>,
}

#[async_trait]
impl KvStore for SimpleKVStore {
    async fn set(&self, request: Request<SetRequest>) -> Result<Response<SetResponse>, Status> {
        todo!()
    }

    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        todo!()
    }

    async fn delete(&self, request: Request<DeleteRequest>) -> Result<Response<DeleteResponse>, Status> {
        todo!()
    }
}
