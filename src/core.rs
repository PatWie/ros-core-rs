extern crate dxr;
use dxr::client::{Call, Client, ClientBuilder, Url};
use paste::paste;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use dxr::server::{async_trait, Handler, HandlerResult};
use dxr::server_axum::axum;
use dxr::server_axum::Server;
use dxr::server_axum::{axum::http::HeaderMap, RouteBuilder};
use dxr::{TryFromParams, TryFromValue, TryToValue, Value};

use crate::client_api::ClientApi;

pub type Services = HashMap<String, HashMap<String, String>>;
pub type Nodes = HashMap<String, String>;
pub type Topics = HashMap<String, String>;
pub type Subscriptions = HashMap<String, HashSet<String>>;
pub type Publishers = HashMap<String, HashSet<String>>;
pub type Parameters = HashMap<String, Value>;
pub type ParameterSubscriptions = HashMap<String, HashMap<String, String>>;

/// An enum that represents the different types of endpoints that can be accessed in the ROS Master API.
///
/// # Variants
///
/// * `RegisterService`: Registers a service with the ROS Master.
/// * `UnRegisterService`: Unregisters a service with the ROS Master.
/// * `RegisterSubscriber`: Registers a subscriber with the ROS Master.
/// * `UnregisterSubscriber`: Unregisters a subscriber with the ROS Master.
/// * `RegisterPublisher`: Registers a publisher with the ROS Master.
/// * `UnregisterPublisher`: Unregisters a publisher with the ROS Master.
/// * `LookupNode`: Looks up a node with the ROS Master.
/// * `GetPublishedTopics`: Gets the published topics from the ROS Master.
/// * `GetTopicTypes`: Gets the topic types from the ROS Master.
/// * `GetSystemState`: Gets the system state from the ROS Master.
/// * `GetUri`: Gets the URI from the ROS Master.
/// * `LookupService`: Looks up a service with the ROS Master.
/// * `DeleteParam`: Deletes a parameter from the ROS Parameter Server.
/// * `SetParam`: Sets a parameter on the ROS Parameter Server.
/// * `GetParam`: Gets a parameter from the ROS Parameter Server.
/// * `SearchParam`: Searches for a parameter on the ROS Parameter Server.
/// * `SubscribeParam`: Subscribes to a parameter on the ROS Parameter Server.
/// * `UnsubscribeParam`: Unsubscribes from a parameter on the ROS Parameter Server.
/// * `HasParam`: Checks if a parameter exists on the ROS Parameter Server.
/// * `GetParamNames`: Gets the names of parameters on the ROS Parameter Server.
/// * `SystemMultiCall`: Performs multiple ROS Master API calls in a single request.
/// * `Default`: The default endpoint used when no other endpoint is specified.
enum MasterEndpoints {
    RegisterService,
    UnRegisterService,
    RegisterSubscriber,
    UnregisterSubscriber,
    RegisterPublisher,
    UnregisterPublisher,
    LookupNode,
    GetPublishedTopics,
    GetTopicTypes,
    GetSystemState,
    GetUri,
    LookupService,
    DeleteParam,
    SetParam,
    GetParam,
    SearchParam,
    SubscribeParam,
    UnsubscribeParam,
    HasParam,
    GetParamNames,
    SystemMultiCall,
    Default,
}

impl MasterEndpoints {
    fn as_str(&self) -> &'static str {
        match self {
            MasterEndpoints::RegisterService => "registerService",
            MasterEndpoints::UnRegisterService => "unregisterService",
            MasterEndpoints::RegisterSubscriber => "registerSubscriber",
            MasterEndpoints::UnregisterSubscriber => "unregisterSubscriber",
            MasterEndpoints::RegisterPublisher => "registerPublisher",
            MasterEndpoints::UnregisterPublisher => "unregisterPublisher",
            MasterEndpoints::LookupNode => "lookupNode",
            MasterEndpoints::GetPublishedTopics => "getPublishedTopics",
            MasterEndpoints::GetTopicTypes => "getTopicTypes",
            MasterEndpoints::GetSystemState => "getSystemState",
            MasterEndpoints::GetUri => "getUri",
            MasterEndpoints::LookupService => "lookupService",
            MasterEndpoints::DeleteParam => "deleteParam",
            MasterEndpoints::SetParam => "setParam",
            MasterEndpoints::GetParam => "getParam",
            MasterEndpoints::SearchParam => "searchParam",
            MasterEndpoints::SubscribeParam => "subscribeParam",
            MasterEndpoints::UnsubscribeParam => "unsubscribeParam",
            MasterEndpoints::HasParam => "hasParam",
            MasterEndpoints::GetParamNames => "getParamNames",
            MasterEndpoints::SystemMultiCall => "system.multicall",
            MasterEndpoints::Default => "",
        }
    }
}

/// Struct containing information about ROS data.
pub struct RosData {
    // RwLocks to allow for concurrent read/write access to data
    service_list: RwLock<Services>, // stores information about available services
    nodes: RwLock<Nodes>,           // stores information about nodes connected to the ROS network
    topics: RwLock<Topics>,         // stores information about available topics
    subscriptions: RwLock<Subscriptions>, // stores information about topic subscriptions
    publications: RwLock<Publishers>, // stores information about topic publishers
    parameters: RwLock<Parameters>, // stores information about ROS parameters
    parameter_subscriptions: RwLock<ParameterSubscriptions>, // stores information about parameter subscriptions
    uri: std::net::SocketAddr,                               // the address of the ROS network
}

pub struct Master {
    data: Arc<RosData>,
}

/// Handler for registering the caller as a provider of the specified service.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `service` - Fully-qualified name of service (string)
/// - `service_api` - ROSRPC Service URI (string)
/// - `caller_api` - XML-RPC URI of caller node (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `ignore` - ignore (integer)
struct RegisterServiceHandler {
    data: Arc<RosData>,
}
type RegisterServiceResponse = (i32, String, i32);
#[async_trait]
impl Handler for RegisterServiceHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("RegisterServiceHandler {:?} ", params);
        type Request = (String, String, String, String);
        let (caller_id, service, service_api, caller_api) = Request::try_from_params(params)?;
        self.data
            .service_list
            .write()
            .unwrap()
            .entry(service)
            .or_default()
            .insert(caller_id.clone(), service_api);

        self.data
            .nodes
            .write()
            .unwrap()
            .insert(caller_id, caller_api);

        Ok((1, String::from(""), 0).try_to_value()?)
    }
}

/// Handler for unregistering the caller as a provider of the specified service.
///
/// # Parameters
///
/// - caller_id - ROS caller ID (string)
/// - service - Fully-qualified name of service (string)
/// - service_api - API URI of service to unregister. Unregistration will only occur if current
/// registration matches. (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - code - response code (integer)
/// - statusMessage - status message (string)
/// - numUnregistered - number of unregistrations (either 0 or 1). If this is zero it means that the
/// caller was not registered as a service provider. The call still succeeds as the intended final
/// state is reached. (integer)
struct UnRegisterServiceHandler {
    data: Arc<RosData>,
}
type UnRegisterServiceResponse = (i32, String, i32);
#[async_trait]
impl Handler for UnRegisterServiceHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("UnRegisterServiceHandler {:?} ", params);
        type Request = (String, String, String);
        let (caller_id, service, _service_api) = Request::try_from_params(params)?;

        let mut service_list = self.data.service_list.write().unwrap();

        let removed = if let Some(providers) = service_list.get_mut(&service) {
            providers.remove(&caller_id);
            providers.is_empty()
        } else {
            false
        };

        if removed {
            service_list.remove(&service);
        }

        Ok((1, "", if removed { 1 } else { 0 }).try_to_value()?)
    }
}

/// Handler for registering the caller as a subscriber to the specified topic.
///
/// # Parameters
///
/// - caller_id - ROS caller ID (string)
/// - topic - Fully-qualified name of the topic (string)
/// - topic_type - Datatype for topic. Must be a package-resource name, i.e. the .msg name (string)
/// - caller_api - API URI of subscriber to register. Will be used for new publisher notifications (string)
///
/// # Returns
///
/// A tuple of integers and a vector of strings representing the response:
///
/// - code - response code (integer)
/// - statusMessage - status message (string)
/// - publishers - a list of XMLRPC API URIs for nodes currently publishing the specified topic (vector of strings).
struct RegisterSubscriberHandler {
    data: Arc<RosData>,
}
type RegisterSubscriberResponse = (i32, String, Vec<String>);
#[async_trait]
impl Handler for RegisterSubscriberHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("RegisterSubscriberHandler {:?} ", params);
        type Request = (String, String, String, String);
        let (caller_id, topic, topic_type, caller_api) = Request::try_from_params(params)?;

        if let Some(known_topic_type) = self.data.topics.read().unwrap().get(&topic.clone()) {
            if known_topic_type != &topic_type {
                let err_msg = format!(
                    "{} for topic {} does not match {}",
                    topic_type, topic, known_topic_type
                );
                return Ok((0, err_msg, "").try_to_value()?);
            }
        }

        self.data
            .subscriptions
            .write()
            .unwrap()
            .entry(topic.clone())
            .or_default()
            .insert(caller_id.clone());
        self.data
            .nodes
            .write()
            .unwrap()
            .insert(caller_id, caller_api);

        let publishers = self
            .data
            .publications
            .read()
            .unwrap()
            .get(&topic)
            .cloned()
            .unwrap_or_default();
        let nodes = self.data.nodes.read().unwrap();
        let publisher_apis: Vec<String> = publishers
            .iter()
            .filter_map(|p| nodes.get(p).cloned())
            .collect();

        return Ok((1, "", publisher_apis).try_to_value()?);
    }
}

/// Handler for unregistering the caller as a publisher of the topic.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `topic` - Fully-qualified name of topic (string)
/// - `caller_api` - API URI of subscriber to unregister. Unregistration will only occur if current
///   registration matches. (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `numUnsubscribed` - number of unsubscriptions (either 0 or 1). If this is zero it means that the caller was not
/// registered as a subscriber. The call still succeeds as the intended final state is reached.
struct UnRegisterSubscriberHandler {
    data: Arc<RosData>,
}
type UnRegisterSubscriberResponse = (i32, String, i32);
#[async_trait]
impl Handler for UnRegisterSubscriberHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("UnRegisterSubscriberHandler {:?} ", params);
        type Request = (String, String, String);
        let (caller_id, topic, _caller_api) = Request::try_from_params(params)?;

        let removed = self
            .data
            .subscriptions
            .write()
            .unwrap()
            .entry(topic.clone())
            .or_default()
            .remove(&caller_id);

        self.data
            .subscriptions
            .write()
            .unwrap()
            .retain(|_, v| !v.is_empty());

        Ok((1, "", if removed { 1 } else { 0 }).try_to_value()?)
    }
}

/// Handler for registering the caller as a publisher of the specified topic.
///
/// # Parameters
///
/// - caller_id - ROS caller ID (string)
/// - topic - Fully-qualified name of topic to register (string)
/// - topic_type - Datatype for topic. Must be a package-resource name, i.e. the .msg name (string)
/// - caller_api - API URI of publisher to register (string)
///
/// # Returns
///
/// A tuple of integers, a string, and a list of strings representing the response:
///
/// - code - response code (integer)
/// - statusMessage - status message (string)
/// - subscriberApis - list of current subscribers of topic in the form of XMLRPC URIs (list of strings)
struct RegisterPublisherHandler {
    data: Arc<RosData>,
}
type RegisterPublisherResponse = (i32, String, Vec<String>);
#[async_trait]
impl Handler for RegisterPublisherHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("RegisterPublisherHandler {:?} ", params);
        type Request = (String, String, String, String);
        let (caller_id, topic, topic_type, caller_api) = Request::try_from_params(params)?;

        if let Some(v) = self.data.topics.read().unwrap().get(&topic.clone()) {
            if v != &topic_type {
                let err_msg = format!("{} for topic {} does not match {}", topic_type, topic, v);
                return Ok((0, err_msg, Vec::<String>::default()).try_to_value()?);
            }
        }

        // TODO(patwie): Maybe holding the lock for a longer time?
        // let mut publications = self.data.publications.write().unwrap();
        self.data
            .publications
            .write()
            .unwrap()
            .entry(topic.clone())
            .or_default()
            .insert(caller_id.clone());
        self.data
            .topics
            .write()
            .unwrap()
            .insert(topic.clone(), topic_type.clone());
        self.data
            .nodes
            .write()
            .unwrap()
            .insert(caller_id.clone(), caller_api.clone());

        let nodes = self.data.nodes.read().unwrap().clone();
        let subscribers_api_urls = self
            .data
            .subscriptions
            .read()
            .unwrap()
            .get(&topic)
            .unwrap_or(&HashSet::new())
            .iter()
            .map(|s| nodes.get(s))
            .filter(|a| a.is_some())
            .map(|a| a.unwrap().to_string())
            .collect::<Vec<String>>();
        let publishers = self
            .data
            .publications
            .read()
            .unwrap()
            .get(&topic)
            .cloned()
            .unwrap_or_default();

        // Inform all subscribers of the new publisher.
        let publisher_apis = publishers.into_iter().collect::<Vec<String>>();
        for client_api_url in subscribers_api_urls.clone() {
            let client_api = ClientApi::new(client_api_url.as_str());
            log::debug!("Call {}", client_api_url);
            let r = client_api
                .publisher_update(&caller_id.as_str(), &topic.as_str(), &publisher_apis)
                .await;
            if let Err(r) = r {
                log::warn!("publisherUpdate call to {} failed: {}", client_api_url, r);
            }
        }

        return Ok((1, "", subscribers_api_urls).try_to_value()?);
    }
}

/// Handler for unregistering the caller as a publisher of the topic.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `topic` - Fully-qualified name of topic to unregister (string)
/// - `caller_api` - API URI of publisher to unregister (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `numUnregistered` - number of unregistrations (either 0 or 1). If this is zero it means that the
/// caller was not registered as a publisher. The call still succeeds as the intended final state is reached.
struct UnRegisterPublisherHandler {
    data: Arc<RosData>,
}
type UnRegisterPublisherResponse = (i32, String, i32);
#[async_trait]
impl Handler for UnRegisterPublisherHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("UnRegisterPublisherHandler {:?} ", params);
        type Request = (String, String, String);
        let (caller_id, topic, caller_api) = Request::try_from_params(params)?;
        println!("Called {caller_id} with {topic} {caller_api}");

        if self
            .data
            .publications
            .write()
            .unwrap()
            .get(&topic.clone())
            .is_none()
        {
            return Ok((1, String::from(""), 0).try_to_value()?);
        }
        let removed = self
            .data
            .publications
            .write()
            .unwrap()
            .entry(topic.clone())
            .or_default()
            .remove(&caller_id);
        self.data
            .publications
            .write()
            .unwrap()
            .retain(|_, v| !v.is_empty());
        Ok((1, "", if removed { 1 } else { 0 }).try_to_value()?)
    }
}

/// Handler for looking up the XML-RPC URI of the node with the associated name/caller_id.
///
/// # Parameters
///
/// - caller_id - ROS caller ID (string)
/// - node_name - Name of node to lookup (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - code - response code (integer)
/// - statusMessage - status message (string)
/// - URI - XML-RPC URI of the node (string). This API is for looking up information about publishers
/// and subscribers. Use lookupService instead to lookup ROS-RPC URIs.
struct LookupNodeHandler {
    data: Arc<RosData>,
}
type LookupNodeResponse = (i32, String, String);
#[async_trait]
impl Handler for LookupNodeHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("LookupNodeHandler {:?} ", params);
        type Request = (String, String);
        let (_caller_id, node_name) = Request::try_from_params(params)?;

        if let Some(node_api) = self.data.nodes.read().unwrap().get(&node_name) {
            return Ok((1, "", node_api).try_to_value()?);
        } else {
            let err_msg = format!("node {} not found", node_name);
            return Ok((0, err_msg, "").try_to_value()?);
        }
    }
}

/// Handler for getting the list of topics that can be subscribed to.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `subgraph` - Restrict topic names to match within the specified subgraph. Subgraph namespace
///   is resolved relative to the caller's namespace. Use empty string to specify all names (string).
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `topics` - a list of lists containing topic names and types, e.g. `[[topic1, type1], [topic2, type2]]`.
/// The list represents topics that can be subscribed to, but not necessarily all topics available in the system.
/// Use `getSystemState()` for a more comprehensive list.
struct GetPublishedTopicsHandler {
    data: Arc<RosData>,
}
type GetPublishedTopicsResponse = (i32, String, Vec<(String, String)>);
#[async_trait]
impl Handler for GetPublishedTopicsHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("GetPublishedTopicsHandler {:?} ", params);
        type Request = (String, String);
        let (_caller_id, _subgraph) = Request::try_from_params(params)?;
        let mut result = Vec::<(String, String)>::new();
        let topics = self.data.topics.read().unwrap().clone();
        for topic in self.data.publications.read().unwrap().keys() {
            let data_type = topics.get(&topic.clone());
            if let Some(data_type) = data_type {
                result.push((topic.clone(), data_type.to_owned()));
            }
        }
        return Ok((1, "", result).try_to_value()?);
    }
}

/// Handler for retrieving the list of topic names and their types.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
///
/// # Returns
///
/// A tuple of integers, a string representing the response, and a list of lists of strings:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `topicTypes` - a list of lists of strings representing the [topicName, topicType] pairs.
struct GetTopicTypesHandler {
    data: Arc<RosData>,
}
type GetTopicTypesResponse = (i32, String, Vec<(String, String)>);
#[async_trait]
impl Handler for GetTopicTypesHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("GetTopicTypesHandler {:?} ", params);
        type Request = String;
        let _caller_id = Request::try_from_params(params)?;
        let result: Vec<_> = self
            .data
            .topics
            .read()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect();
        return Ok((1, "", result).try_to_value()?);
    }
}

/// Handler for retrieving a list representation of the ROS system state.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `systemState` - a list of tuples representing the system state:
///     - Each tuple contains two elements: a string representing the topic/service name, and a list of strings
///       representing the associated publishers/subscribers/servers/clients.
///     - The first tuple contains the list of publishers, the second contains the list of subscribers, and the third
///       contains the list of services.
struct GetSystemStateHandler {
    data: Arc<RosData>,
}
type GetSystemStateResponse = (i32, String, Vec<(String, Vec<String>)>);
#[async_trait]
impl Handler for GetSystemStateHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("GetSystemStateHandler {:?} ", params);
        type Request = String;
        let _caller_id = Request::try_from_params(params)?;
        let publishers: Vec<(String, Vec<String>)> = self
            .data
            .publications
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                let mut node_names: Vec<_> = v.iter().cloned().collect();
                node_names.sort();

                (k.clone(), node_names)
            })
            .collect();
        let subscribers: Vec<(String, Vec<String>)> = self
            .data
            .subscriptions
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                let mut node_names: Vec<_> = v.iter().cloned().collect();
                node_names.sort();

                (k.clone(), node_names)
            })
            .collect();
        let services: Vec<(String, Vec<String>)> = self
            .data
            .service_list
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                let mut node_names: Vec<_> = v.keys().cloned().collect();
                node_names.sort();

                (k.clone(), node_names)
            })
            .collect();
        return Ok((1, "", (publishers, subscribers, services)).try_to_value()?);
    }
}

/// Handler for getting the URI of the master.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `masterURI` - URI of the ROS master (string)
struct GetUriHandler {
    data: Arc<RosData>,
}
type GetUriResponse = (i32, String, String);
#[async_trait]
impl Handler for GetUriHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("GetUriHandler {:?} ", params);
        type Request = String;
        let _caller_id = Request::try_from_params(params)?;
        let result = format!("/{}", self.data.uri.clone());
        return Ok((1, "", (result,)).try_to_value()?);
    }
}

/// Handler for looking up all providers of a particular service.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `service` - Fully-qualified name of service (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `serviceUrl` - URL that provides the address and port of the service. The function fails if there
/// is no provider.
struct LookupServiceHandler {
    data: Arc<RosData>,
}
type LookupServiceResponse = (i32, String, String);
#[async_trait]
impl Handler for LookupServiceHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("LookupServiceHandler {:?} ", params);
        type Request = (String, String);
        let (_caller_id, service) = Request::try_from_params(params)?;

        let services = self
            .data
            .service_list
            .read()
            .unwrap()
            .get(&service)
            .cloned();
        if services.is_some() {
            let services = services.unwrap();
            if services.is_empty() {
                return Ok((
                    0,
                    "`no providers for service \"{service}\"`".to_string(),
                    "",
                )
                    .try_to_value()?);
            } else {
                let service_url = services.values().next().unwrap();
                return Ok((1, "".to_string(), service_url.clone()).try_to_value()?);
            }
        }

        return Ok((
            0,
            "`no providers for service \"{service}\"`".to_string(),
            "",
        )
            .try_to_value()?);
    }
}

/// Handler for deleting a parameter.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `key` - Parameter name (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `ignore` - an integer indicating the number of parameters deleted. This is always 0, since a delete
/// operation deletes only one parameter.
struct DeleteParamHandler {
    data: Arc<RosData>,
}
type DeleteParamResponse = (i32, String, i32);
#[async_trait]
impl Handler for DeleteParamHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("DeleteParamHandler {:?} ", params);
        type Request = (String, String);
        let (_caller_id, key) = Request::try_from_params(params)?;
        self.data.parameters.write().unwrap().remove(&key);
        return Ok((1, "", 0).try_to_value()?);
    }
}

/// Handler for setting a ROS parameter.
///
/// # Parameters
///
/// - caller_id - ROS caller ID (string)
/// - key - Parameter name (string)
/// - value - Parameter value. If it's a dictionary, it will be treated as a parameter tree, where
/// the key is the parameter namespace. For example {'x':1,'y':2,'sub':{'z':3}} will set
/// key/x=1, key/y=2, and key/sub/z=3. Furthermore, it will replace all existing parameters
/// in the key parameter namespace with the parameters in value. You must set parameters individually
/// if you wish to perform a union update (XMLRPCLegalValue)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - code - response code (integer)
/// - statusMessage - status message (string)
/// - ignore - ignored (integer). Returns 0 in all cases.
struct SetParamHandler {
    data: Arc<RosData>,
}
type SetParamResponse = (i32, String, i32);
#[async_trait]
impl Handler for SetParamHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("SetParamHandler {:?} ", params);
        type Request = (String, String, Value);
        let (caller_id, key, value) = Request::try_from_params(params)?;
        self.data
            .parameters
            .write()
            .unwrap()
            .insert(key.clone(), value.clone());

        // TODO(patwie): handle case where value is not a single value.
        let all_key_values;
        all_key_values = vec![(key.clone(), value.clone())];

        for (cur_key, cur_value) in all_key_values {
            // Update the parameter value
            self.data
                .parameters
                .write()
                .unwrap()
                .insert(cur_key.clone(), cur_value.clone());
            // Notify any parameter subscribers about this new value
            let subscribers = self
                .data
                .parameter_subscriptions
                .read()
                .unwrap()
                .get(&cur_key)
                .cloned();

            if let Some(subscribers) = subscribers {
                for client_api_url in subscribers.values() {
                    let client_api = ClientApi::new(client_api_url.as_str());
                    log::debug!("Call {}", client_api_url);
                    let r = client_api
                        .param_update(&caller_id.as_str(), &cur_key.as_str(), &cur_value)
                        .await;
                    if let Err(r) = r {
                        log::warn!("paramUpdate call to {} failed: {}", client_api_url, r);
                    }
                }
            }
        }

        Ok((1, "", 0).try_to_value()?)
    }
}

/// Handler for retrieving a parameter value from the server.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `key` - Parameter name. If `key` is a namespace, `getParam()` will return a parameter tree (string).
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `parameterValue` - the value of the requested parameter (of type `XMLRPCLegalValue`). If `code` is not 1,
/// `parameterValue` should be ignored. If `key` is a namespace, the return value will be a dictionary, where each
/// key is a parameter in that namespace. Sub-namespaces are also represented as dictionaries.
struct GetParamHandler {
    data: Arc<RosData>,
}
type GetParamResponse = (i32, String, Value);
#[async_trait]
impl Handler for GetParamHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("GetParamHandler {:?} ", params);
        type Request = (String, String);
        let (_caller_id, key) = Request::try_from_params(params)?;
        Ok(match self.data.parameters.read().unwrap().get(&key) {
            Some(value) => (1, "", value.to_owned()),
            None => (0, "", Value::string("")),
        }
        .try_to_value()?)
    }
}

/// Handler for subscribing to a parameter value and updates.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `caller_api` - Node API URI of subscriber for paramUpdate callbacks (string)
/// - `key` - Parameter name (string)
///
/// # Returns
///
/// A tuple of integers, a string representing the response, and the parameter value:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `parameterValue` - the parameter value (XML-RPC legal value). If the parameter has not been set yet,
/// the value will be an empty dictionary.
struct SubscribeParamHandler {
    data: Arc<RosData>,
}
type SubscribeParamResponse = (i32, String, Value);
#[async_trait]
impl Handler for SubscribeParamHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("SubscribeParamHandler {:?} ", params);
        type Request = (String, String, String);
        let (caller_id, caller_api, key) = Request::try_from_params(params)?;
        let param_subscriptions = &mut self.data.parameter_subscriptions.write().unwrap();
        if !param_subscriptions.contains_key(&key) {
            param_subscriptions.insert(key.clone(), HashMap::new());
        }
        let subscriptions = param_subscriptions.get_mut(&key).unwrap();

        subscriptions.insert(caller_id.clone(), caller_api.clone());

        let value = self
            .data
            .parameters
            .read()
            .unwrap()
            .get(&key)
            .cloned()
            .unwrap_or(Value::string(""));
        Ok((1, "", value).try_to_value()?)
    }
}

/// Handler for unsubscribing from a parameter and its updates.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `caller_api` - Node API URI of subscriber (string)
/// - `key` - Parameter name to unsubscribe from (string)
///
/// # Returns
///
/// A tuple of integers and a string representing the response:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `numUnsubscribed` - number of unsubscriptions (either 0 or 1). If this is zero it means that the
/// caller was not subscribed to the parameter. The call still succeeds as the intended final state is reached.
struct UnSubscribeParamHandler {
    data: Arc<RosData>,
}
type UnSubscribeParamResponse = (i32, String, i32);
#[async_trait]
impl Handler for UnSubscribeParamHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("UnSubscribeParamHandler {:?} ", params);
        type Request = (String, String, String);
        let (caller_id, _caller_api, key) = Request::try_from_params(params)?;

        let mut parameter_subscriptions = self.data.parameter_subscriptions.write().unwrap();
        let subscribers = parameter_subscriptions.entry(key.clone()).or_default();
        let removed = subscribers.remove(&caller_id).is_some();
        if subscribers.is_empty() {
            parameter_subscriptions.remove(&key);
        }
        Ok((1, "", if removed { 1 } else { 0 }).try_to_value()?)
    }
}

/// Handler for checking if a parameter is stored on the server.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
/// - `key` - Parameter name (string)
///
/// # Returns
///
/// A tuple of integers and a boolean representing the response:
///
/// - `code` - Response code (integer)
/// - `statusMessage` - Status message (string)
/// - `hasParam` - Boolean indicating whether the parameter is stored on the server (true) or not (false).
struct HasParamHandler {
    data: Arc<RosData>,
}
type HasParamResponse = (i32, String, bool);
#[async_trait]
impl Handler for HasParamHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("HasParamHandler {:?} ", params);
        type Request = (String, String);
        let (_caller_id, key) = Request::try_from_params(params)?;
        let has = self.data.parameters.read().unwrap().contains_key(&key);
        Ok((1, "", has).try_to_value()?)
    }
}

/// Handler for getting a list of all parameter names stored on the server.
///
/// # Parameters
///
/// - `caller_id` - ROS caller ID (string)
///
/// # Returns
///
/// A tuple of integers, a string, and a list of parameter names:
///
/// - `code` - response code (integer)
/// - `statusMessage` - status message (string)
/// - `parameterNameList` - list of all parameter names stored on the server (list of strings)
struct GetParamNamesHandler {
    data: Arc<RosData>,
}
type GetParamNamesResponse = (i32, String, Vec<String>);
#[async_trait]
impl Handler for GetParamNamesHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("GetParamNamesHandler {:?} ", params);
        type Request = (String, String);
        let _caller_id = Request::try_from_params(params)?;
        let keys: Vec<String> = self
            .data
            .parameters
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        Ok((1, "", keys).try_to_value()?)
    }
}

/// Handler for debugging output. This handler logs the incoming request parameters as a debug
/// message and always returns a success response with an empty status message.
///
/// # Parameters
///
/// - `data` - Shared reference to `RosData` struct (Arc<RosData>)
///
/// # Returns
///
/// A `HandlerResult` representing a tuple of integers and a string:
///
/// - `code` - response code (integer)
/// - `ignore` ignored (string, always empty in this case)
/// - `ignore` ignored (string, always empty in this case)
struct DebugOutputHandler {
    #[allow(dead_code)]
    data: Arc<RosData>,
}
#[async_trait]
impl Handler for DebugOutputHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("SystemMultiCallHandler {:?} ", params);
        Ok((1, "", "").try_to_value()?)
    }
}

macro_rules! make_handlers {
    ($self:ident, $($endpoint:expr=>$handlerFn:ident),*) => {{
        let router = RouteBuilder::new()
            $(.add_method($endpoint.as_str(), Box::new($handlerFn {
                data: $self.data.clone(),
            })))*
            .build();
        router
    }};
}

impl Master {
    pub fn new(url: &std::net::SocketAddr) -> Master {
        Master {
            data: Arc::new(RosData {
                service_list: RwLock::new(Services::new()),
                nodes: RwLock::new(Nodes::new()),
                topics: RwLock::new(Topics::new()),
                subscriptions: RwLock::new(Subscriptions::new()),
                publications: RwLock::new(Publishers::new()),
                parameters: RwLock::new(Parameters::new()),
                parameter_subscriptions: RwLock::new(ParameterSubscriptions::new()),
                uri: url.to_owned(),
            }),
        }
    }

    fn create_router(&self) -> axum::Router {
        let router = make_handlers!(
            self,
            MasterEndpoints::RegisterService => RegisterServiceHandler,
            MasterEndpoints::UnRegisterService => UnRegisterServiceHandler,
            MasterEndpoints::RegisterSubscriber => RegisterSubscriberHandler,
            MasterEndpoints::UnregisterSubscriber => UnRegisterSubscriberHandler,
            MasterEndpoints::RegisterPublisher => RegisterPublisherHandler,
            MasterEndpoints::UnregisterPublisher => UnRegisterPublisherHandler,
            MasterEndpoints::LookupNode => LookupNodeHandler,
            MasterEndpoints::GetPublishedTopics => GetPublishedTopicsHandler,
            MasterEndpoints::GetTopicTypes => GetTopicTypesHandler,
            MasterEndpoints::GetSystemState => GetSystemStateHandler,
            MasterEndpoints::GetUri => GetUriHandler,
            MasterEndpoints::LookupService => LookupServiceHandler,
            MasterEndpoints::DeleteParam => DeleteParamHandler,
            MasterEndpoints::SetParam => SetParamHandler,
            MasterEndpoints::GetParam => GetParamHandler,
            MasterEndpoints::SearchParam => GetParamHandler,
            MasterEndpoints::SubscribeParam => SubscribeParamHandler,
            MasterEndpoints::UnsubscribeParam => UnSubscribeParamHandler,
            MasterEndpoints::HasParam => HasParamHandler,
            MasterEndpoints::GetParamNames => GetParamNamesHandler,
            MasterEndpoints::SystemMultiCall => DebugOutputHandler,
            MasterEndpoints::Default => DebugOutputHandler
        );
        router
    }

    /// Starts the ROS core server and listens for incoming requests.
    ///
    /// The server will listen on the URI specified during the construction of `RosCoreServer`.
    /// The server router will handle requests to both `/` and `/RPC2`.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating if the server started successfully or if there was an error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ros_core_rs::core::Master;
    /// use url::Url;
    ///
    /// let socket_address = ros_core_rs::url_to_socket_addr(&Url::parse("http://0.0.0.0:11311").unwrap());
    /// let core = Master::new(&socket_address.unwrap());
    /// core.serve();
    /// ```
    pub async fn serve(&self) -> anyhow::Result<()> {
        // Some ROS implementation use /RPC2 like the python subscribers. Some ROS implementation
        // use / like Foxglove. We serve them all.
        let router: axum::Router = axum::Router::new()
            .nest("/", self.create_router())
            .nest("/RPC2", self.create_router());
        log::info!("roscore-rs is listening on {}", self.data.uri);
        let server = Server::from_route(self.data.uri, router);
        server.serve().await
    }
}

pub struct MasterClient {
    client: Client,
}

macro_rules! implement_client_fn {
    ($name:ident($($v:ident: $t:ty),*)->$response_type:ident) => {
        paste!{
            pub async fn [<$name:snake>](&self, $($v: $t),*) -> anyhow::Result<$response_type>{
                let request = Call::new(
                    MasterEndpoints::$name.as_str(),
                    ($($v,)*),
                );
                let response = self.client.call(request).await?;
                let value = $response_type::try_from_value(&response)?;
                Ok(value)
            }
        }
    };
}

macro_rules! make_client{
    ($($name:tt($($v:ident: $t:ty),*)-> $response_type:ident),*) => {
        $(implement_client_fn!($name($($v: $t),*)-> $response_type);)*

    }

}

impl MasterClient {
    /// Constructs a new instance of `MasterClient` with the provided `Url`.
    ///
    /// # Arguments
    ///
    /// * `url` - A `Url` struct representing the ROS master URI to connect to.
    ///
    /// # Example
    ///
    /// ```
    /// use ros_core_rs::core::MasterClient;
    /// use url::Url;
    ///
    /// let uri = Url::parse("http://localhost:11311").unwrap();
    /// let client = MasterClient::new(&uri);
    /// ```
    pub fn new(url: &Url) -> Self {
        let client = ClientBuilder::new(url.clone())
            .user_agent("master-client")
            .build();
        Self { client }
    }

    make_client!(
        RegisterService(caller_id: &str, service: &str, service_api: &str, caller_api: &str) -> RegisterServiceResponse,
        UnRegisterService(caller_id: &str, service: &str, service_api:  &str) -> UnRegisterServiceResponse,
        RegisterSubscriber(caller_id: &str, topic: &str, topic_type: &str, caller_api: &str) -> RegisterSubscriberResponse,
        UnregisterSubscriber(caller_id: &str, topic: &str, caller_api: &str) -> UnRegisterSubscriberResponse,
        RegisterPublisher(caller_id: &str, topic: &str, topic_type: &str, caller_api: &str) -> RegisterPublisherResponse,
        UnregisterPublisher(caller_id: &str, topic: &str, caller_api: &str) -> UnRegisterPublisherResponse,
        LookupNode(caller_id: &str, node_name: &str) -> LookupNodeResponse,
        GetPublishedTopics(caller_id: &str, subgraph: &str) -> GetPublishedTopicsResponse,
        GetTopicTypes(caller_id: &str) -> GetTopicTypesResponse,
        GetSystemState(caller_id: &str) -> GetSystemStateResponse,
        GetUri(caller_id: &str) -> GetUriResponse,
        LookupService(caller_id: &str, service: &str) -> LookupServiceResponse,
        DeleteParam(caller_id: &str, key: &str) -> DeleteParamResponse,
        // TODO():  correct args
        SetParam(caller_id: &str, key: &str, value: &Value) -> SetParamResponse,
        GetParam(caller_id: &str, key: &str) -> GetParamResponse,
        SearchParam(caller_id: &str, key: &str) -> GetParamResponse,
        // TODO():  correct args
        SubscribeParam(caller_id: &str, caller_api: &str, keys: &str) -> SubscribeParamResponse,
        UnsubscribeParam(caller_id: &str, caller_api: &str, key: &str) -> UnSubscribeParamResponse,
        HasParam(caller_id: &str, key: &str) -> HasParamResponse,
        GetParamNames(caller_id: &str) -> GetParamNamesResponse
    );
}
