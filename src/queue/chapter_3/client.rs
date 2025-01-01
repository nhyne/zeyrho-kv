use zeyrho::zeyrho::queue::queue_client::QueueClient;
use zeyrho::zeyrho::queue::EnqueueRequest;

pub async fn execute_queries() -> Result<Vec<String>, tonic::transport::Error> {
    let mut client = QueueClient::connect("http://localhost:8080").await?;

    let request = tonic::Request::new(EnqueueRequest { number: 1000 });

    let response = client.enqueue(request).await.unwrap();

    println!("RESPONSE: {}", response.get_ref().confirmation);

    Ok(Vec::new())
}
