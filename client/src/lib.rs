use std::str::FromStr;

use anyhow::Context;
use reqwest::Method;
use tokio_stream::StreamExt;
use tonic::{
    transport::{Channel, Uri},
    Status,
};
use tracing::error;

mod grpc;

pub async fn start(periscope_url: Uri, homeassistant_url: Uri) -> anyhow::Result<()> {
    let mut periscope_client = grpc::inner_client::InnerClient::connect(periscope_url).await?;
    let response = periscope_client.stream_requests(grpc::Void {}).await?;
    let mut stream = response.into_inner();

    let homeassistant_client = reqwest::Client::new();

    while let Some(request) = stream.next().await {
        if let Err(err) = dispatch(
            request,
            &homeassistant_url,
            &homeassistant_client,
            &mut periscope_client,
        )
        .await
        {
            error!("{err}");
        }
    }
    Ok(())
}

async fn dispatch(
    request: Result<grpc::IncomingRequest, Status>,
    homeassistant_url: &Uri,
    homeassistant_client: &reqwest::Client,
    periscope_client: &mut grpc::inner_client::InnerClient<Channel>,
) -> anyhow::Result<()> {
    let grpc::IncomingRequest {
        id,
        uri,
        method,
        headers,
        body,
    } = request.context("Received error")?;

    let method = Method::from_bytes(method.as_bytes()).context("Invalid method")?;

    let mut url = reqwest::Url::from_str(&uri).context("Invalid uri")?;
    if url
        .set_scheme(
            homeassistant_url
                .scheme()
                .map(|s| s.as_str())
                .unwrap_or("http"),
        )
        .is_err()
    {
        anyhow::bail!("Invalid scheme {homeassistant_url}");
    }
    url.set_host(homeassistant_url.host())
        .context("Invalid host")?;
    if url
        .set_port(homeassistant_url.port().map(|p| p.as_u16()))
        .is_err()
    {
        anyhow::bail!("Invalid port {homeassistant_url}");
    }

    let mut builder = homeassistant_client.request(method, url);
    for header in headers {
        builder = builder.header(header.name, header.value);
    }
    let response = builder.body(body).send().await?;
    let status = response.status().as_u16() as u32;
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| grpc::Header {
            name: k.as_str().as_bytes().to_owned(),
            value: v.as_bytes().to_owned(),
        })
        .collect();
    let body = response.bytes().await?.to_vec();

    periscope_client
        .send_response(grpc::OutgoingResponse {
            id,
            status,
            headers,
            body,
        })
        .await?;

    Ok(())
}
