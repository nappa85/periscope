use tokio::sync::oneshot::Sender;

pub type ChannelItem = (common::IncomingRequest, Sender<common::OutgoingResponse>);

pub mod grpc;
pub mod tls;
pub mod webserver;
