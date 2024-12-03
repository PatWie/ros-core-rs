extern crate dxr;
use dxr_client::{Client, ClientBuilder, Url};
use maplit::hashmap;
use paste::paste;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tokio::task::JoinSet;
use uuid::Context;

use dxr_server::{async_trait, Handler, HandlerResult};
use dxr_server::{
    axum::{self, http::HeaderMap},
    RouteBuilder, Server,
};

use dxr::{TryFromParams, TryFromValue, TryToValue, Value};

use crate::client_api::ClientApi;
use crate::param_tree::ParamValue;

pub type Services = HashMap<String, HashMap<String, String>>;
pub type Nodes = HashMap<String, String>;
pub type Topics = HashMap<String, String>;
pub type Subscriptions = HashMap<String, HashSet<String>>;
pub type Publishers = HashMap<String, HashSet<String>>;
pub type Parameters = crate::param_tree::ParamValue;

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
    GetPid,
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
            MasterEndpoints::GetPid => "getPid",
            MasterEndpoints::Default => "",
        }
    }
}

#[derive(Debug)]
struct ParamSubscription {
    node_id: String,
    param: String,
    api_uri: String,
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
    parameter_subscriptions: RwLock<Vec<ParamSubscription>>, // stores information about parameter subscriptions
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

        let service = resolve(&caller_id, &service);

        self.data
            .service_list
            .write()
            .unwrap()
            .entry(service)
            .or_default()
            .insert(caller_id.clone(), service_api);

        register_node(&self.data.nodes, &caller_id, &caller_api).await;

        Ok((1, String::from(""), 0).try_to_value()?)
    }
}

async fn register_node(nodes : &RwLock<Nodes>, caller_id: &str, caller_api : &str) -> () {
    let shutdown_api_url;
    {
        let mut nodes = nodes.write().unwrap();
        match nodes.entry(caller_id.to_owned()) {
            Entry::Vacant(v) => {
                v.insert(caller_api.to_owned());
                return
            },
            Entry::Occupied(mut e) => {
                let e = e.get_mut();
                if e == caller_api {
                    return
                } else {
                    shutdown_api_url = std::mem::replace(e, caller_api.to_owned());
                }
            }
        }
    }
    let res = shutdown_node(&shutdown_api_url, caller_id).await;
    if let Err(e) = res {
        log::warn!("Error shutting down previous instance of node '{caller_id}': {e:?}. New node will be registered regardless. Check for stray processes.");
    }
}

async fn shutdown_node(client_api_url: &str, node_id : &str) -> anyhow::Result<()> {
    let client_api = ClientApi::new(client_api_url);
    let res = client_api.shutdown("/master", &format!("[{}] Reason: new node registered with same name", node_id)).await;
    res
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

        let service = resolve(&caller_id, &service);

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

        let topic = resolve(&caller_id, &topic);

        if let Some(known_topic_type) = self.data.topics.read().unwrap().get(&topic.clone()) {
            if known_topic_type != &topic_type && topic_type != "*" {
                log::warn!("Topic '{topic}' was initially published as '{known_topic_type}', but subscriber '{caller_id}' wants it as '{topic_type}'.");
            }
        }

        self.data
            .subscriptions
            .write()
            .unwrap()
            .entry(topic.clone())
            .or_default()
            .insert(caller_id.clone());
        
        register_node(&self.data.nodes, &caller_id, &caller_api).await;

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

        let topic = resolve(&caller_id, &topic);

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

        let topic = resolve(&caller_id, &topic);

        if let Some(v) = self.data.topics.read().unwrap().get(&topic.clone()) {
            if v != &topic_type {
                log::warn!("New publisher for topic '{topic}' has type '{topic_type}', but it is already published as '{v}'.");
            }
        }

        register_node(&self.data.nodes, &caller_id, &caller_api).await;

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
        let publisher_nodes = publishers.into_iter().collect::<Vec<String>>();
        let publisher_apis = self
            .data
            .nodes
            .read() // Note: This should not be a race condition, because for every publisher, the node has to be there first, and we're reading "nodes" after "publishers".
            .unwrap()
            .iter()
            .filter(|node| publisher_nodes.contains(node.0))
            .map(|node| node.1.clone())
            .collect::<Vec<String>>();
        for client_api_url in subscribers_api_urls.clone() {
            let client_api = ClientApi::new(client_api_url.as_str());
            log::debug!("Call {}", client_api_url);
            let r = client_api
                .publisher_update(&caller_id.as_str(), &topic.as_str(), &publisher_apis)
                .await;
            match r {
                Err(e) => log::warn!("publisherUpdate call to {} failed: {}", client_api_url, e),
                Ok(v) => log::debug!("publisherUpdate call to {} succeeded, returning: {:?}", client_api_url, v)
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

        let topic = resolve(&caller_id, &topic);

        log::debug!("Called {caller_id} with {topic} {caller_api}");

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

/// Handler for getting the PID of the master.
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
/// - `pid` - PID of the ROS master (integer)
struct GetPidHandler {
    #[allow(unused)]
    data: Arc<RosData>,
}
type GetPidResponse = (i32, String, i32);
#[async_trait]
impl Handler for GetPidHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("GetPidHandler {:?} ", params);
        type Request = String;
        let _caller_id = Request::try_from_params(params)?;
        let result = std::process::id() as i32; // max pid on linux is 2^22, so the typecast should have no unintended side effects
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
        let (caller_id, service) = Request::try_from_params(params)?;

        let service = resolve(&caller_id, &service);

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
        let (caller_id, key) = Request::try_from_params(params)?;
        let key = resolve(&caller_id, &key);
        let key = key.strip_prefix('/').unwrap_or(&key).split('/');
        self.data.parameters.write().unwrap().remove(key);
        return Ok((1, "", 0).try_to_value()?);
    }
}

fn one_is_prefix_of_the_other(a: &str, b: &str) -> bool {
    let len = a.len().min(b.len());
    a[..len] == b[..len]
}

async fn update_client_with_new_param_value(
    client_api_url: String,
    updating_node_id: String,
    subscribing_node_id: String,
    param_name: String,
    new_value: Value,
) -> Result<Value, anyhow::Error> {
    let client_api = ClientApi::new(&client_api_url);
    let request = client_api.param_update(&updating_node_id, &param_name, &new_value);
    let res = request.await;
    match res {
        Ok(ref v) => log::debug!(
            "Sent new value for param '{}' to node '{}'. response: {:?}",
            param_name,
            subscribing_node_id,
            &v
        ),
        Err(ref e) => log::debug!(
            "Error sending new value for param '{}' to node '{}': {:?}",
            param_name,
            subscribing_node_id,
            e
        ),
    }

    Ok(res?)
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
        let key = resolve(&caller_id, &key);

        let mut update_futures = JoinSet::new();

        {
            let key = key.clone();
            let mut params = self.data.parameters.write().unwrap();
            let key_split = key.strip_prefix('/').unwrap_or(&key).split('/');
            params.update_inner(key_split, value);

            let param_subscriptions = self.data.parameter_subscriptions.read().unwrap();
            log::info!("updating param {}", &key);
            for subscription in param_subscriptions.iter() {
                log::debug!(
                    "subscriber {:?} has subscription? {}",
                    &subscription,
                    one_is_prefix_of_the_other(&key, &subscription.param)
                );
                if one_is_prefix_of_the_other(&key, &subscription.param) {
                    let subscribed_key_spit = subscription
                        .param
                        .strip_prefix('/')
                        .unwrap_or(&subscription.param)
                        .split('/');
                    let new_value = params.get(subscribed_key_spit).unwrap();
                    update_futures.spawn(update_client_with_new_param_value(
                        subscription.api_uri.clone(),
                        caller_id.clone(),
                        subscription.node_id.clone(),
                        subscription.param.clone(),
                        new_value,
                    ));
                }
            }
        }

        while let Some(res) = update_futures.join_next().await {
            match res {
                Ok(Ok(v)) => {
                    log::debug!("a subscriber has been updated (res: {:#?})", &v);
                }
                Ok(Err(err)) => {
                    log::warn!(
                        "Error updating a subscriber of changed param {}:\n{:#?}",
                        &key,
                        err
                    );
                }
                Err(err) => {
                    log::warn!(
                        "Error updating a subscriber of changed param {}:\n{:#?}",
                        &key,
                        err
                    );
                }
            }
        }

        log::info!("done updating subscribers");

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
        let (caller_id, key) = Request::try_from_params(params)?;
        let key_full = resolve(&caller_id, &key);
        let params = self.data.parameters.read().unwrap();
        let key_path = key_full.strip_prefix('/').unwrap_or(&key_full).split('/');

        Ok(match params.get(key_path) {
            Some(value) => (1, format!("Parameter [{}]", &key_full), value.to_owned()),
            None => (-1, format!("Parameter [{}] is not set", &key_full), Value::i4(0)),
        }
        .try_to_value()?)
    }
}

struct SearchParamHandler {
    data: Arc<RosData>,
}
type SearchParamResponse = (i32, String, Value);
#[async_trait]
impl Handler for SearchParamHandler {
    async fn handle(&self, params: &[Value], _headers: HeaderMap) -> HandlerResult {
        log::debug!("SearchParamHandler {:?} ", params);
        type Request = (String, String);
        let (caller_id, key) = Request::try_from_params(params)?;

        let mut param_name = String::with_capacity(caller_id.len() + key.len());

        // For an explanation of what the search algorithm does, see the comment in the original code:
        // https://github.com/ros/ros_comm/blob/9ae132c/tools/rosmaster/src/rosmaster/paramserver.py#L82
        let params = self.data.parameters.read().unwrap().get_keys();
        let key = key.strip_prefix('/').unwrap_or(&key);
        let key_first_element = key.split('/').next().unwrap_or("");
        let namespace = caller_id
            .strip_prefix('/')
            .unwrap_or(&caller_id)
            .split('/')
            .collect::<Vec<&str>>();

        let range = (0usize..namespace.len()).rev();

        for up_to in range {
            param_name.clear();
            param_name.push('/');
            for idx in 0..up_to {
                param_name.push_str(namespace[idx]);
                param_name.push('/');
            }
            param_name.push_str(key_first_element);
            if params.contains(&param_name) {
                break;
            }
        }

        for path in key.split('/').skip(1) {
            param_name.push('/');
            param_name.push_str(path);
        }

        Ok((1, "", param_name).try_to_value()?)
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
        let key = resolve(&caller_id, &key);

        register_node(&self.data.nodes, &caller_id, &caller_api).await;

        let mut new_subscription = Some(ParamSubscription {
            node_id: caller_id.clone(),
            param: key.clone(),
            api_uri: caller_api,
        });

        {
            // RwLock scope
            let param_subscriptions = &mut self.data.parameter_subscriptions.write().unwrap();

            // replace old entry if subscribing node has restarted
            for subscription in param_subscriptions.iter_mut() {
                if &subscription.node_id == &caller_id && &subscription.param == &key {
                    let _ = std::mem::replace(subscription, new_subscription.take().unwrap());
                    break;
                }
            }

            // add a new entry if it's a new node id
            if let Some(new_subscription) = new_subscription {
                param_subscriptions.push(new_subscription)
            }
        }

        let key_split = key.strip_prefix('/').unwrap_or(&key).split('/');

        let value = self
            .data
            .parameters
            .read()
            .unwrap()
            .get(key_split)
            .unwrap_or(Value::string("".to_owned()));
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
        let (caller_id, caller_api, key) = Request::try_from_params(params)?;
        let key = resolve(&caller_id, &key);

        let mut parameter_subscriptions = self.data.parameter_subscriptions.write().unwrap();
        let mut removed = false;
        parameter_subscriptions.retain(|subscription| {
            if subscription.api_uri == caller_api && subscription.param == key {
                removed = true;
                false
            } else {
                true
            }
        });
        Ok((1, "", if removed { 1 } else { 0 }).try_to_value()?)
    }
}

fn resolve(caller_id: &str, key: &str) -> String {
    match key.chars().next() {
        None => "".to_owned(),
        Some('/') => key.to_owned(),
        Some('~') => format!("{}/{}", caller_id, &key[1..]),
        Some(_) => match caller_id.rsplit_once('/') {
            Some((namespace, _node_name)) => format!("{}/{}", namespace, key),
            None => key.to_owned(),
        },
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
        let (caller_id, key) = Request::try_from_params(params)?;
        let key = resolve(&caller_id, &key);
        let has = self
            .data
            .parameters
            .read()
            .unwrap()
            .get_keys()
            .contains(&key);
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
        let a = <(String, String)>::try_from_params(params);
        let b = <(String,)>::try_from_params(params);

        if a.is_err() && b.is_err() {
            a?;
        }

        let keys: Vec<String> = self.data.parameters.read().unwrap().get_keys();
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

fn get_node_id() -> Option<[u8; 6]> {
    let ip_link = std::process::Command::new("ip")
        .arg("link")
        .output()
        .ok()?
        .stdout;
    let ip_link = String::from_utf8_lossy(&ip_link);
    let mut next_is_mac = false;
    let mut mac = None;
    for element in ip_link.split_whitespace() {
        if next_is_mac {
            mac = Some(element);
            break;
        }
        if element == "link/ether" {
            next_is_mac = true;
        }
    }
    let mac = mac?;
    let mut all_ok = true;
    let mac: Vec<u8> = mac
        .split(':')
        .filter_map(|hex| {
            let res = u8::from_str_radix(hex, 16);
            all_ok &= res.is_ok();
            res.ok()
        })
        .collect();
    if !all_ok {
        return None;
    }
    let mac: [u8; 6] = mac.try_into().ok()?;
    Some(mac)
}

impl Master {
    pub fn new(url: &std::net::SocketAddr) -> Master {
        let run_id = ParamValue::Value(Value::string(
            uuid::Uuid::new_v1(
                uuid::Timestamp::now(Context::new_random()),
                &get_node_id().unwrap_or_default(),
            )
            .to_string(),
        ));
        Master {
            data: Arc::new(RosData {
                service_list: RwLock::new(Services::new()),
                nodes: RwLock::new(Nodes::new()),
                topics: RwLock::new(Topics::new()),
                subscriptions: RwLock::new(Subscriptions::new()),
                publications: RwLock::new(Publishers::new()),
                parameters: RwLock::new(Parameters::HashMap(hashmap! {
                    "run_id".to_owned() => run_id
                })),
                parameter_subscriptions: RwLock::new(Vec::new()),
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
            MasterEndpoints::SearchParam => SearchParamHandler,
            MasterEndpoints::SubscribeParam => SubscribeParamHandler,
            MasterEndpoints::UnsubscribeParam => UnSubscribeParamHandler,
            MasterEndpoints::HasParam => HasParamHandler,
            MasterEndpoints::GetParamNames => GetParamNamesHandler,
            MasterEndpoints::SystemMultiCall => DebugOutputHandler,
            MasterEndpoints::GetPid => GetPidHandler,
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
        let server = Server::from_route(router);
        Ok(server.serve(self.data.uri.try_into()?).await?)
    }
}

pub struct MasterClient {
    client: Client,
}

macro_rules! implement_client_fn {
    ($name:ident($($v:ident: $t:ty),*)->$response_type:ident) => {
        paste!{
            pub async fn [<$name:snake>](&self, $($v: $t),*) -> anyhow::Result<$response_type>{
                let request = (
                    MasterEndpoints::$name.as_str(),
                    ($($v,)*),
                );
                let response = self.client.call(request.0, request.1).await?;
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
        GetPid(caller_id: &str) -> GetPidResponse,
        LookupService(caller_id: &str, service: &str) -> LookupServiceResponse,
        DeleteParam(caller_id: &str, key: &str) -> DeleteParamResponse,
        // TODO():  correct args
        SetParam(caller_id: &str, key: &str, value: &Value) -> SetParamResponse,
        GetParam(caller_id: &str, key: &str) -> GetParamResponse,
        SearchParam(caller_id: &str, key: &str) -> SearchParamResponse,
        // TODO():  correct args
        SubscribeParam(caller_id: &str, caller_api: &str, keys: &str) -> SubscribeParamResponse,
        UnsubscribeParam(caller_id: &str, caller_api: &str, key: &str) -> UnSubscribeParamResponse,
        HasParam(caller_id: &str, key: &str) -> HasParamResponse,
        GetParamNames(caller_id: &str) -> GetParamNamesResponse
    );
}
