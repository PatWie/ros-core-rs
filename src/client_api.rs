use dxr::client::{Call, Client, ClientBuilder, Url};
use dxr::Value;

pub struct ClientApi {
    client: Client,
}

impl ClientApi {
    pub fn new(uri: &str) -> Self {
        let url = Url::parse(uri).expect("Failed to parse client-api URL.");
        let client = ClientBuilder::new(url.clone())
            .user_agent("ros-core-rs-client-api")
            .build();
        Self { client }
    }
    pub async fn publisher_update(
        &self,
        caller_id: &str,
        topic: &str,
        publisher_apis: &Vec<String>,
    ) -> anyhow::Result<()> {
        let request = Call::new("publisherUpdate", (caller_id, topic, publisher_apis));
        let result = self.client.call(request).await;
        result
    }
    pub async fn param_update(
        &self,
        caller_id: &str,
        key: &str,
        value: &Value,
    ) -> anyhow::Result<()> {
        let request = Call::new("paramUpdate", (caller_id, key, value));
        let result = self.client.call(request).await;
        result
    }
}
