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

pub struct RosData {
    service_list: RwLock<Services>,
    nodes: RwLock<Nodes>,
    topics: RwLock<Topics>,
    subscriptions: RwLock<Subscriptions>,
    publications: RwLock<Publishers>,
    parameters: RwLock<Parameters>,
    parameter_subscriptions: RwLock<ParameterSubscriptions>,
    uri: std::net::SocketAddr,
}

pub struct Master {
    data: Arc<RosData>,
}

// registerService(caller_id, service, service_api, caller_api)
//
// Register the caller as a provider of the specified service.
//
// Parameters
//
// caller_id (str) ROS caller ID
// service (str) Fully-qualified name of service
// service_api (str) ROSRPC Service URI
// caller_api (str) XML-RPC URI of caller node
//
// Returns (int, str, int) (code, statusMessage, ignore)
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

// unregisterService(caller_id, service, service_api)
//
// Unregister the caller as a provider of the specified service.
//
// Parameters
//
// caller_id (str) ROS caller ID
// service (str) Fully-qualified name of service
// service_api (str) API URI of service to unregister. Unregistration will only occur if current
//     registration matches.
//
// Returns (int, str, int) (code, statusMessage, numUnregistered).
//
// Number of unregistrations (either 0 or 1). If this is zero it means that the caller was not
// registered as a service provider. The call still succeeds as the intended final state is
// reached.
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

        if let Some(providers) = self.data.service_list.write().unwrap().get_mut(&service) {
            let removed = providers.remove(&caller_id).is_some();
            self.data
                .service_list
                .write()
                .unwrap()
                .retain(|_, v| !v.is_empty());
            return Ok((1, "", if removed { 1 } else { 0 }).try_to_value()?);
        } else {
            return Ok((1, "", 0).try_to_value()?);
        }
    }
}

// registerSubscriber(caller_id, topic, topic_type, caller_api)
//
// Subscribe the caller to the specified topic. In addition to receiving a list of current
// publishers, the subscriber will also receive notifications of new publishers via the
// publisherUpdate API.
//
// Parameters
//
// caller_id (str) ROS caller ID
// topic (str) Fully-qualified name of topic.
// topic_type (str) Datatype for topic. Must be a package-resource name, i.e. the .msg name.
// caller_api (str) API URI of subscriber to register. Will be used for new publisher notifications.
//
// Returns (int, str, [str]) (code, statusMessage, publishers)
//
// Publishers is a list of XMLRPC API URIs for nodes currently publishing the specified topic.
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

// unregisterSubscriber(caller_id, topic, caller_api)
//
// Unregister the caller as a publisher of the topic.
//
// Parameters
//
// caller_id (str) ROS caller ID
// topic (str) Fully-qualified name of topic.
// caller_api (str) API URI of subscriber to unregister. Unregistration will only occur if current registration matches.
//
// Returns (int, str, int) (code, statusMessage, numUnsubscribed)
//
// If numUnsubscribed is zero it means that the caller was not registered as a subscriber. The call
// still succeeds as the intended final state is reached.
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

// registerPublisher(caller_id, topic, topic_type, caller_api)
//
// Register the caller as a publisher the topic.
//
// Parameters
//
// caller_id (str) ROS caller ID
// topic (str) Fully-qualified name of topic to register.
// topic_type (str) Datatype for topic. Must be a package-resource name, i.e. the .msg name.
// caller_api (str) API URI of publisher to register.
//
// Returns (int, str, [str]) (code, statusMessage, subscriberApis)
//
// List of current subscribers of topic in the form of XMLRPC URIs.
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

// unregisterPublisher(caller_id, topic, caller_api)
//
// Unregister the caller as a publisher of the topic.
//
// Parameters
//
// caller_id (str) ROS caller ID
// topic (str) Fully-qualified name of topic to unregister.
// caller_api (str) API URI of publisher to unregister. Unregistration will only occur if current
//      registration matches.
//
// Returns (int, str, int) (code, statusMessage, numUnregistered)
//
// If numUnregistered is zero it means that the caller was not registered as a publisher. The call
// still succeeds as the intended final state is reached.
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

// lookupNode(caller_id, node_name)
//
// Get the XML-RPC URI of the node with the associated name/caller_id. This API is for looking
// information about publishers and subscribers. Use lookupService instead to lookup ROS-RPC URIs.
//
// Parameters
//
// caller_id (str) ROS caller ID
// node (str) Name of node to lookup
//
// Returns (int, str, str) (code, statusMessage, URI)
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

// getPublishedTopics(caller_id, subgraph)
//
// Get list of topics that can be subscribed to. This does not return topics that have no
// publishers. See getSystemState() to get more comprehensive list.
//
// Parameters
//
// caller_id (str) ROS caller ID
// subgraph (str) Restrict topic names to match within the specified subgraph. Subgraph namespace
//      is resolved relative to the caller's namespace. Use empty string to specify all names.
//
// Returns (int, str, [[str, str],]) (code, statusMessage, [ [topic1, type1]...[topicN, typeN] ])
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

// getTopicTypes(caller_id)
//
// Retrieve list topic names and their types.
//
// Parameters
//rosout
// caller_id (str) ROS caller ID
//
// Returns (int, str, [ [str,str] ]) (code, statusMessage, topicTypes)
//
// topicTypes is a list of [topicName, topicType] pairs.
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

// getSystemState(caller_id)
//
// Retrieve list representation of system state (i.e. publishers, subscribers, and services).
//
// Parameters
//
// caller_id (str) ROS caller ID
//
// Returns (int, str, [ [str,[str] ], [str,[str] ], [str,[str] ] ]) (code, statusMessage, systemState)
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

// getUri(caller_id)
//
// Get the URI of the master.
//
// Parameters
//
// caller_id (str) ROS caller ID
//
// Returns (int, str, str) (code, statusMessage, masterURI)
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

// lookupService(caller_id, service)
//
// Lookup all provider of a particular service.
//
// Parameters
//
// caller_id (str) ROS caller ID
// service (str) Fully-qualified name of service
//
// Returns (int, str, str) (code, statusMessage, serviceUrl)
//
// service URL is provides address and port of the service. Fails if there is no provider.
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

// deleteParam(caller_id, key)
// Delete parameter
//
// Parameters
// caller_id (str) ROS caller ID
// key (str) Parameter name.
//
// Returns (int, str, int) (code, statusMessage, ignore)
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

// setParam(caller_id, key, value)
//
// Set parameter. NOTE: if value is a dictionary it will be treated as a parameter tree, where key
// is the parameter namespace. For example {'x':1,'y':2,'sub':{'z':3}} will set key/x=1, key/y=2,
// and key/sub/z=3. Furthermore, it will replace all existing parameters in the key parameter
// namespace with the parameters in value. You must set parameters individually if you wish to
// perform a union update.
//
// Parameters
//
// caller_id (str) ROS caller ID
// key (str) Parameter name.
// value (XMLRPCLegalValue) Parameter value.
//
// Returns (int, str, int) (code, statusMessage, ignore)
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

// getParam(caller_id, key)
//
// Retrieve parameter value from server.
//
// Parameters
//
// caller_id (str) ROS caller ID
// key (str) Parameter name. If key is a namespace, getParam() will return a parameter tree.
//
// Returns (int, str, XMLRPCLegalValue) (code, statusMessage, parameterValue)
//
// If code is not 1, parameterValue should be ignored. If key is a namespace, the return value will
// be a dictionary, where each key is a parameter in that namespace. Sub-namespaces are also
// represented as dictionaries.
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

// subscribeParam(caller_id, caller_api, key)
//
// Retrieve parameter value from server and subscribe to updates to that param. See paramUpdate()
// in the Node API.
//
// Parameters
//
// caller_id (str) ROS caller ID.
// key (str) Parameter name
// caller_api (str) Node API URI of subscriber for paramUpdate callbacks.
//
// Returns (int, str, XMLRPCLegalValue) (code, statusMessage, parameterValue)
//
// If code is not 1, parameterValue should be ignored. parameterValue is an empty dictionary if the
// parameter has not been set yet.
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

// unsubscribeParam(caller_id, caller_api, key)
//
// Retrieve parameter value from server and subscribe to updates to that param. See paramUpdate()
// in the Node API.
//
// Parameters
//
// caller_id (str) ROS caller ID.
// key (str) Parameter name.
// caller_api (str) Node API URI of subscriber.
//
// Returns (int, str, int) (code, statusMessage, numUnsubscribed)
//
// If numUnsubscribed is zero it means that the caller was not subscribed to the parameter.
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

        let removed = self
            .data
            .parameter_subscriptions
            .write()
            .unwrap()
            .entry(key.clone())
            .or_default()
            .remove(&caller_id)
            .is_some();
        self.data
            .parameter_subscriptions
            .write()
            .unwrap()
            .retain(|_, v| !v.is_empty());
        Ok((1, "", if removed { 1 } else { 0 }).try_to_value()?)
    }
}

// hasParam(caller_id, key)
//
// Check if parameter is stored on server.
//
// Parameters
//
// caller_id (str) ROS caller ID.
// key (str) Parameter name.
//
// Returns (int, str, bool) (code, statusMessage, hasParam)
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

// getParamNames(caller_id)
//
// Get list of all parameter names stored on this server.
//
// Parameters
//
// caller_id (str) ROS caller ID.
//
// Returns (int, str, [str]) (code, statusMessage, parameterNameList)
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

struct DebugOutputHandler {
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
    ($name:ident($($v:ident),*)-> $response_type:ident) => {
        paste!{
        pub async fn [<$name:snake>](&self, $($v: &str),*) -> anyhow::Result<$response_type>{

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
    ($($name:tt($($v:ident),*)-> $response_type:ident),*) => {
        $(implement_client_fn!($name($($v),*)-> $response_type);)*

    }

}

impl MasterClient {
    pub fn new(uri: &str) -> Self {
        let url = Url::parse(uri).expect("Failed to parse  URL.");
        let client = ClientBuilder::new(url.clone())
            .user_agent("dxr-client-example")
            .build();
        Self { client }
    }

    make_client!(
        RegisterService(caller_id, service, service_api, caller_api)->RegisterServiceResponse,
        UnRegisterService(caller_id, service, service_api)->UnRegisterServiceResponse,
        RegisterSubscriber(caller_id, topic, topic_type, caller_api)->RegisterSubscriberResponse,
        UnregisterSubscriber(caller_id, topic, caller_api)->UnRegisterSubscriberResponse,
        RegisterPublisher(caller_id, topic, topic_type, caller_api)->RegisterPublisherResponse,
        UnregisterPublisher(caller_id, topic, caller_api)->UnRegisterPublisherResponse,
        LookupNode(caller_id, node_name)->LookupNodeResponse,
        GetPublishedTopics(caller_id, subgraph)->GetPublishedTopicsResponse,
        GetTopicTypes(caller_id)->GetTopicTypesResponse,
        GetSystemState(caller_id)->GetSystemStateResponse,
        GetUri(caller_id)->GetUriResponse,
        LookupService(caller_id, service)->LookupServiceResponse,
        DeleteParam(caller_id, key)->DeleteParamResponse,
        // TODO(): correct args
        SetParam(caller_id, key, value)->SetParamResponse,
        GetParam(caller_id, key)->GetParamResponse,
        SearchParam(caller_id, key)->GetParamResponse,
        // TODO(): correct args
        SubscribeParam(caller_id, caller_api, keys)->SubscribeParamResponse,
        UnsubscribeParam(caller_id, caller_api, key)->UnSubscribeParamResponse,
        HasParam(caller_id, key)->HasParamResponse,
        GetParamNames(caller_id)->GetParamNamesResponse
    );
}
