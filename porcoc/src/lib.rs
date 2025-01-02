use std::{borrow::Cow, str::FromStr};

use prost::bytes::Bytes;
use reqwest::StatusCode;
use tokio_stream::StreamExt;
use tonic::{
    transport::{Certificate, ClientTlsConfig, Uri},
    Status,
};
use tracing::{debug, error};

mod grpc;

pub async fn start(
    certs: Option<Certificate>,
    porco_url: Uri,
    target_url: Uri,
) -> anyhow::Result<()> {
    let mut endpoint = tonic::transport::Endpoint::new(porco_url)?;
    if let Some(certs) = certs {
        endpoint = endpoint.tls_config(ClientTlsConfig::new().ca_certificate(certs))?;
    }
    let client = endpoint.connect().await?;
    let mut porco_client = grpc::inner_client::InnerClient::new(client);
    let response = porco_client.stream_requests(common::grpc::Void {}).await?;
    let mut stream = response.into_inner();

    let target_client = reqwest::Client::new();

    while let Some(request) = stream.next().await {
        let res = dispatch(
            request,
            &target_url,
            &target_client,
        )
        .await
        .unwrap_or_else(|(id, error)| {
            (id.unwrap_or_default(), common::OutgoingResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                headers: vec![],
                body: Bytes::from_iter(error.bytes()),
            })
        });
        let request = common::grpc::OutgoingResponse::from(res);
        if let Err(err)= porco_client.send_response(request).await {
            error!("{err}");
        }
    }
    Ok(())
}

async fn dispatch(
    request: Result<common::grpc::IncomingRequest, Status>,
    target_url: &Uri,
    target_client: &reqwest::Client,
) -> Result<(u64, common::OutgoingResponse), (Option<u64>, Cow<'static, str>)> {
    debug!("Dispatching {request:?}");
    let request = request.map_err(|err| (None, Cow::Owned(format!("Received error: {err}"))))?;
    let id = request.id;
    let common::IncomingRequest {
        uri,
        method,
        headers,
        body,
    } = common::IncomingRequest::try_from(request).map_err(|err| (Some(id), Cow::Owned(format!("Conversion error: {err}"))))?;

    // do we really have to re-parse the Url?
    let mut url = reqwest::Url::from_str(&target_url.to_string()).map_err(|_| (Some(id), Cow::Borrowed("Invalid uri")))?;
    url.set_path(uri.path());
    url.set_query(uri.query());
    //url.set_fragment(uri.fragment());

    let mut builder = target_client.request(method, url);
    for (k,v) in headers {
        builder = builder.header(k,v);
    }
    let response = builder.body(body).send().await.map_err(|err| (Some(id), Cow::Owned(format!("Call error: {err}"))))?;

    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let body = response.bytes().await.map_err(|err| (Some(id), Cow::Owned(format!("Body error: {err}"))))?;

    Ok((id, common::OutgoingResponse {status, headers,
        body
    }))
}
