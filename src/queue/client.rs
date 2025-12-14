use zeyrho::zeyrho::queue::EnqueueRequest;
use zeyrho::zeyrho::queue::queue_client::QueueClient;

pub async fn execute_queries() -> Result<Vec<String>, tonic::transport::Error> {
    let mut client = QueueClient::connect("http://localhost:8080").await?;

    let request = tonic::Request::new(EnqueueRequest {
        payload: Vec::from("1000".as_bytes()),
    });

    let response = client.enqueue(request).await.unwrap();

    println!("RESPONSE: {}", response.get_ref().message_id);

    Ok(Vec::new())
}
