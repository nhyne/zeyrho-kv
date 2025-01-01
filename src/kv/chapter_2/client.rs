use zeyrho::zeyrho::kv_store::kv_store_client::KvStoreClient;
use zeyrho::zeyrho::kv_store::SetRequest;

pub async fn execute_queries() -> Result<Vec<String>, tonic::transport::Error> {
    let mut client = KvStoreClient::connect("http://localhost:8080").await?;

    let request = tonic::Request::new(SetRequest {
        key: "Something".to_string(),
        value: 1000,
    });

    let response = client.set(request).await.unwrap();

    println!("RESPONSE: {}", response.get_ref().confirmation);

    Ok(Vec::new())
}
