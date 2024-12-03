use dxr::Value;
use dxr_client::{Client, ClientBuilder, Url};

pub struct ClientApi {
    client: Client,
}

impl ClientApi {
    /// Creates a new `ClientApi` instance with the given URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - A string slice representing the URI of the client API.
    ///
    /// # Returns
    ///
    /// A new `ClientApi` instance.
    pub fn new(uri: &str) -> Self {
        // Parse the URI and create a new `Client` instance.
        let url = Url::parse(uri).expect("Failed to parse client-api URL.");
        let client = ClientBuilder::new(url.clone())
            .user_agent("ros-core-rs-client-api")
            .build();
        Self { client }
    }

    /// Sends a "publisherUpdate" request to the ROS node.
    ///
    /// # Arguments
    ///
    /// * `caller_id` - A string slice representing the ID of the caller.
    /// * `topic` - A string slice representing the name of the topic.
    /// * `publisher_apis` - A vector of string slices representing the API URLs of the publishers.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating whether the request was successful.
    pub async fn publisher_update(
        &self,
        caller_id: &str,
        topic: &str,
        publisher_apis: &Vec<String>,
    ) -> anyhow::Result<Value> {
        let result = self.client.call::<_, _>("publisherUpdate", (caller_id, topic, publisher_apis)).await;
        
        Ok(result?)
    }

    /// Sends a "paramUpdate" request to the ROS node.
    ///
    /// # Arguments
    ///
    /// * `caller_id` - A string slice representing the ID of the caller.
    /// * `key` - A string slice representing the name of the parameter.
    /// * `value` - A `Value` representing the new value of the parameter.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating whether the request was successful.
    pub async fn param_update(
        &self,
        caller_id: &str,
        key: &str,
        value: &Value,
    ) -> anyhow::Result<Value> {
        let result = self.client.call("paramUpdate", (caller_id, key, value)).await;
        Ok(result?)
    }

    /// Requests the node to shut down
    ///
    /// # Arguments
    ///
    /// * `caller_id` - A string slice representing the ID of the caller.
    /// * `reason` - Reason for shutting the node down. Will likely show up in logs.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating whether the request was successful.

    pub async fn shutdown(
        &self,
        caller_id: &str,
        reason: &str,
    ) -> anyhow::Result<()> {
        let result = self.client.call("shutdown", (caller_id, reason)).await;
        Ok(result?)
    }
}
