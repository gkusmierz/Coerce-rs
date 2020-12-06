use crate::actor::RemoteRequest;
use crate::cluster::node::RemoteNode;
use crate::context::RemoteActorSystem;

use crate::net::client::RemoteClientStream;
use crate::net::message::SessionEvent;
use coerce_rt::actor::message::Message;

use uuid::Uuid;

pub struct SetSystem(pub RemoteActorSystem);

impl Message for SetSystem {
    type Result = ();
}

pub struct GetNodes;

impl Message for GetNodes {
    type Result = Vec<RemoteNode>;
}

pub struct PushRequest(pub Uuid, pub RemoteRequest);

impl Message for PushRequest {
    type Result = ();
}

pub struct PopRequest(pub Uuid);

impl Message for PopRequest {
    type Result = Option<RemoteRequest>;
}

pub struct RegisterClient<T: RemoteClientStream>(pub Uuid, pub T);

impl<T: RemoteClientStream> Message for RegisterClient<T>
where
    T: 'static + Sync + Send,
{
    type Result = ();
}

pub struct RegisterNodes(pub Vec<RemoteNode>);

impl Message for RegisterNodes {
    type Result = ();
}

pub struct RegisterNode(pub RemoteNode);

impl Message for RegisterNode {
    type Result = ();
}

pub struct ClientWrite(pub Uuid, pub SessionEvent);

impl Message for ClientWrite {
    type Result = ();
}
