// Copyright (c) 2023 LogCraft, SAS.
// SPDX-License-Identifier: MPL-2.0

use http_body_util::BodyExt;
use tokio::{net::TcpStream, time::timeout};
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiView};
use wasmtime_wasi_http::{
    bindings::http::types::ErrorCode,
    body::HyperOutgoingBody,
    hyper_request_error,
    io::TokioIo,
    types::{HostFutureIncomingResponse, IncomingResponse, OutgoingRequestConfig},
    WasiHttpCtx, WasiHttpView,
};

pub struct State {
    pub table: ResourceTable,
    pub ctx: WasiCtx,
    pub http: WasiHttpCtx,
}

impl State {
    pub fn new() -> Self {
        Self {
            table: ResourceTable::new(),
            ctx: WasiCtx::builder().build(),
            http: WasiHttpCtx::new(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl WasiView for State {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl WasiHttpView for State {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<HostFutureIncomingResponse> {
        Ok(default_send_request_test(request, config))
    }
}

pub fn default_send_request_test(
    request: hyper::Request<HyperOutgoingBody>,
    config: OutgoingRequestConfig,
) -> HostFutureIncomingResponse {
    let handle = wasmtime_wasi::runtime::spawn(async move {
        Ok(default_send_request_handler_test(request, config).await)
    });
    HostFutureIncomingResponse::pending(handle)
}

pub(crate) fn dns_error(rcode: String, info_code: u16) -> ErrorCode {
    ErrorCode::DnsError(wasmtime_wasi_http::bindings::http::types::DnsErrorPayload {
        rcode: Some(rcode),
        info_code: Some(info_code),
    })
}

// ! Quick fix to allow invalid certificate (for selfsigned certificates)
pub async fn default_send_request_handler_test(
    mut request: hyper::Request<HyperOutgoingBody>,
    OutgoingRequestConfig {
        use_tls,
        connect_timeout,
        first_byte_timeout,
        between_bytes_timeout,
    }: OutgoingRequestConfig,
) -> Result<IncomingResponse, ErrorCode> {
    let authority = if let Some(authority) = request.uri().authority() {
        if authority.port().is_some() {
            authority.to_string()
        } else {
            let port = if use_tls { 443 } else { 80 };
            format!("{}:{port}", authority)
        }
    } else {
        return Err(ErrorCode::HttpRequestUriInvalid);
    };
    let tcp_stream = timeout(connect_timeout, TcpStream::connect(&authority))
        .await
        .map_err(|_| ErrorCode::ConnectionTimeout)?
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AddrNotAvailable => {
                dns_error("address not available".to_string(), 0)
            }

            _ => {
                if e.to_string()
                    .starts_with("failed to lookup address information")
                {
                    dns_error("address not available".to_string(), 0)
                } else {
                    ErrorCode::ConnectionRefused
                }
            }
        })?;

    let (mut sender, worker) = if use_tls {
        #[cfg(any(target_arch = "riscv64", target_arch = "s390x"))]
        {
            return Err(crate::bindings::http::types::ErrorCode::InternalError(
                Some("unsupported architecture for SSL".to_string()),
            ));
        }

        #[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
        {
            let mut native_tls_builder = tokio_native_tls::native_tls::TlsConnector::builder();
            native_tls_builder.danger_accept_invalid_certs(true);

            let native_tls_connector: tokio_native_tls::native_tls::TlsConnector =
                native_tls_builder.build().unwrap();

            let connector = tokio_native_tls::TlsConnector::from(native_tls_connector);

            let mut parts = authority.split(':');
            let host = parts.next().unwrap_or(&authority);

            let stream = connector.connect(host, tcp_stream).await.unwrap();

            let stream = TokioIo::new(stream);

            let (sender, conn) = timeout(
                connect_timeout,
                hyper::client::conn::http1::handshake(stream),
            )
            .await
            .map_err(|_| ErrorCode::ConnectionTimeout)?
            .map_err(hyper_request_error)?;

            let worker = wasmtime_wasi::runtime::spawn(async move {
                match conn.await {
                    Ok(()) => {}
                    // TODO: shouldn't throw away this error and ideally should
                    // surface somewhere.
                    Err(e) => tracing::warn!("dropping error {e}"),
                }
            });

            (sender, worker)
        }
    } else {
        let stream = TokioIo::new(tcp_stream);

        let (sender, conn) = timeout(
            connect_timeout,
            // TODO: we should plumb the builder through the http context, and use it here
            hyper::client::conn::http1::handshake(stream),
        )
        .await
        .map_err(|_| ErrorCode::ConnectionTimeout)?
        .map_err(hyper_request_error)?;

        let worker = wasmtime_wasi::runtime::spawn(async move {
            match conn.await {
                Ok(()) => {}
                // TODO: same as above, shouldn't throw this error away.
                Err(e) => tracing::warn!("dropping error {e}"),
            }
        });

        (sender, worker)
    };

    // at this point, the request contains the scheme and the authority, but
    // the http packet should only include those if addressing a proxy, so
    // remove them here, since SendRequest::send_request does not do it for us
    *request.uri_mut() = http::Uri::builder()
        .path_and_query(
            request
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or("/"),
        )
        .build()
        .expect("comes from valid request");

    let resp = timeout(first_byte_timeout, sender.send_request(request))
        .await
        .map_err(|_| ErrorCode::ConnectionReadTimeout)?
        .map_err(hyper_request_error)?
        .map(|body| body.map_err(hyper_request_error).boxed());

    Ok(IncomingResponse {
        resp,
        worker: Some(worker),
        between_bytes_timeout,
    })
}
