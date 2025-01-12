use rand::prelude::*;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Mutex;
use std::time;
use tokio::sync::mpsc;
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::codegen::tokio_stream::Stream;
use tonic::{async_trait, transport::Server, Request, Response, Status, Streaming};
use zeyrho::zeyrho::queue::queue_server::{Queue, QueueServer};
use zeyrho::zeyrho::queue::{
    DequeueRequest, DequeueResponse, EnqueueRequest, EnqueueResponse, ReplicateDataRequest,
    ReplicateDataResponse, SizeRequest, SizeResponse,
};
use opentelemetry::{
    global,
    trace::{Tracer, TracerProvider as _},
};
use opentelemetry_sdk::trace::TracerProvider;

mod proto {
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("queue_descriptor");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = "127.0.0.1:8080".parse().unwrap();

    let provider = TracerProvider::builder()
        .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
        .build();
    let tracer = provider.tracer("readme_example");

    tracer.in_span("doing_work", |cx| {
        println!("anything in here");
    });

    // Shutdown trace pipeline
    // provider.shutdown().expect("TracerProvider should shutdown successfully");

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let queue_service = SimpleQueue {
        queue: Mutex::new(VecDeque::new()),
    };

    Server::builder()
        .add_service(service)
        .add_service(QueueServer::new(queue_service))
        .serve(address)
        .await?;
    Ok(())
}

struct SimpleQueue {
    queue: Mutex<VecDeque<i32>>,
}

#[async_trait]
impl Queue for SimpleQueue {
    async fn enqueue(
        &self,
        request: Request<EnqueueRequest>,
    ) -> Result<Response<EnqueueResponse>, Status> {
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
        for n in 0..num_to_pop {
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
        let (tx, rx) = mpsc::channel(10);

        let _ = tokio::spawn(async move {
            for i in 1..10 {
                let message = ReplicateDataResponse {
                    message_id: "cool".to_string(),
                    message_data: vec![],
                    next_offset: i,
                };

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                if tx.send(Ok(message)).await.is_err() {
                    println!("Client disconnected");
                    break;
                }
            }
        });

        let outbound = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(outbound) as Self::ReplicateDataStream
        ))
    }
}
