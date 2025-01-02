use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

use tokio::sync::{
    broadcast::{self, Receiver, Sender},
    mpsc, oneshot, Mutex, MutexGuard,
};
use tokio_stream::{wrappers::BroadcastStream, Stream};
use tonic::{Request, Response, Status};
use tracing::error;

tonic::include_proto!("inner");

#[derive(Debug)]
pub struct Inner {
    id_manager: IdManager,
    broacast_tx: Sender<common::grpc::IncomingRequest>,
}

impl Inner {
    pub fn new(mut request_tx: mpsc::Receiver<crate::ChannelItem>) -> Self {
        let id_manager = IdManager::default();

        // convert `mpsc` into `broadcast`
        let (broacast_tx, _broadcast_rx) = broadcast::channel(1);
        tokio::spawn({
            let id_manager = id_manager.clone();
            let broacast_tx = broacast_tx.clone();
            async move {
                while let Some((request, oneshot_tx)) = request_tx.recv().await {
                    let mut id_manager = id_manager.lock().await;
                    let id = id_manager.inc_id();
                    id_manager.receivers.insert(id, oneshot_tx);
                    if let Err(err) =
                        broacast_tx.send(common::grpc::IncomingRequest::from((id, request)))
                    {
                        error!("{err}");
                    }
                }
            }
        });

        Self {
            id_manager,
            broacast_tx,
        }
    }
}

#[tonic::async_trait]
impl inner_server::Inner for Inner {
    type StreamRequestsStream = StreamRequestsStream;

    async fn stream_requests(
        &self,
        _: Request<common::grpc::Void>,
    ) -> Result<Response<Self::StreamRequestsStream>, Status> {
        Ok(Response::new(StreamRequestsStream::new(
            self.id_manager.clone(),
            self.broacast_tx.subscribe(),
        )))
    }

    async fn send_response(
        &self,
        request: Request<common::grpc::OutgoingResponse>,
    ) -> Result<Response<common::grpc::Void>, Status> {
        let response = request.into_inner();
        let oneshot_tx = {
            let mut id_manager = self.id_manager.lock().await;
            id_manager
                .receivers
                .remove(&response.id)
                .ok_or_else(|| Status::invalid_argument("Invalid request id"))?
        };
        let response = common::OutgoingResponse::try_from(response)?;
        if oneshot_tx.send(response).is_err() {
            return Err(Status::deadline_exceeded("Timed out"));
        }
        Ok(Response::new(common::grpc::Void {}))
    }
}

#[derive(Debug, Default, Clone)]
struct IdManager(Arc<Mutex<IdManagerInner>>);

impl IdManager {
    async fn lock(&self) -> MutexGuard<'_, IdManagerInner> {
        self.0.lock().await
    }
}

#[derive(Debug)]
struct IdManagerInner {
    next_id: u64,
    receivers: HashMap<u64, oneshot::Sender<common::OutgoingResponse>>,
}

impl Default for IdManagerInner {
    fn default() -> Self {
        // id 0 is used on error
        Self {
            next_id: 1,
            receivers: HashMap::default(),
        }
    }
}

impl IdManagerInner {
    fn inc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

pin_project_lite::pin_project! {
    pub struct StreamRequestsStream {
        id_manager: IdManager,
        #[pin]
        stream: BroadcastStream<common::grpc::IncomingRequest>,
    }
}

impl StreamRequestsStream {
    fn new(id_manager: IdManager, request_rx: Receiver<common::grpc::IncomingRequest>) -> Self {
        Self {
            id_manager,
            stream: BroadcastStream::new(request_rx),
        }
    }
}

impl Stream for StreamRequestsStream {
    type Item = Result<common::grpc::IncomingRequest, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match ready!(this.stream.poll_next(cx)) {
            None => Poll::Ready(None),
            Some(res) => Poll::Ready(Some(
                res.map_err(|err| Status::resource_exhausted(err.to_string())),
            )),
        }
    }
}
