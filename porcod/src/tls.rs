use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};
use tokio_rustls::server::TlsStream;

pin_project_lite::pin_project! {
    #[project = TlsProj]
    pub enum Tls {
        Rustls { #[pin] stream: TlsStream<TcpStream> },
        None { #[pin] stream: TcpStream },
    }
}

impl AsyncRead for Tls {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this: TlsProj<'_> = self.project();
        match this {
            TlsProj::Rustls { stream } => stream.poll_read(cx, buf),
            TlsProj::None { stream } => stream.poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Tls {
    fn is_write_vectored(&self) -> bool {
        match self {
            Self::Rustls { stream } => stream.is_write_vectored(),
            Self::None { stream } => stream.is_write_vectored(),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this: TlsProj<'_> = self.project();
        match this {
            TlsProj::Rustls { stream } => stream.poll_flush(cx),
            TlsProj::None { stream } => stream.poll_flush(cx),
        }
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this: TlsProj<'_> = self.project();
        match this {
            TlsProj::Rustls { stream } => stream.poll_shutdown(cx),
            TlsProj::None { stream } => stream.poll_shutdown(cx),
        }
    }
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let this: TlsProj<'_> = self.project();
        match this {
            TlsProj::Rustls { stream } => stream.poll_write(cx, buf),
            TlsProj::None { stream } => stream.poll_write(cx, buf),
        }
    }
    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        let this: TlsProj<'_> = self.project();
        match this {
            TlsProj::Rustls { stream } => stream.poll_write_vectored(cx, bufs),
            TlsProj::None { stream } => stream.poll_write_vectored(cx, bufs),
        }
    }
}
