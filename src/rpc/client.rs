use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};
use futures::AsyncReadExt;

use super::convert::{fill_frame_builder, frame_from_reader};
use super::endpoint::Endpoint;
use super::read_con_capnp::read_con_service;
use super::{set_compatibility_stamp, validate_compatibility_stamp};
use crate::types::ConFrame;

/// A synchronous RPC client that wraps the Cap'n Proto async transport.
pub struct RpcClient {
    endpoint: Endpoint,
    runtime: tokio::runtime::Runtime,
}

async fn connect_service(
    endpoint: &Endpoint,
) -> Result<read_con_service::Client, Box<dyn std::error::Error>> {
    match endpoint {
        Endpoint::Tcp(addr) => {
            let stream = tokio::net::TcpStream::connect(addr).await?;
            stream.set_nodelay(true)?;
            Ok(bootstrap_client(stream))
        }
        #[cfg(unix)]
        Endpoint::Unix(path) => {
            let stream = tokio::net::UnixStream::connect(path).await?;
            Ok(bootstrap_client(stream))
        }
    }
}

fn bootstrap_client<S>(stream: S) -> read_con_service::Client
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + 'static,
{
    let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
    let network = twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    );
    let mut rpc_system = RpcSystem::new(Box::new(network), None);
    let service: read_con_service::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    tokio::task::spawn_local(rpc_system);
    service
}

impl RpcClient {
    /// Creates a new RPC client targeting TCP `host:port` or Unix `unix:/path`.
    pub fn new(addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            endpoint: Endpoint::parse(addr)?,
            runtime,
        })
    }

    /// Parses a file by sending its contents to the RPC server.
    pub fn parse_file(
        &self,
        path: &std::path::Path,
    ) -> Result<Vec<ConFrame>, Box<dyn std::error::Error>> {
        let data = std::fs::read(path)?;
        self.parse_bytes(&data)
    }

    /// Parses raw file bytes via the RPC server and rebuilds frames from Cap'n Proto.
    pub fn parse_bytes(&self, data: &[u8]) -> Result<Vec<ConFrame>, Box<dyn std::error::Error>> {
        self.runtime.block_on(async {
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async {
                    let service = connect_service(&self.endpoint).await?;

                    let mut request = service.parse_frames_request();
                    {
                        let mut parse_request = request.get().init_req();
                        set_compatibility_stamp(parse_request.reborrow().init_compatibility());
                        parse_request.set_file_contents(data);
                    }
                    let response = request.send().promise.await?;
                    let result = response.get()?.get_result()?;
                    validate_compatibility_stamp(result.get_compatibility()?)?;
                    let frame_data_list = result.get_frames()?;

                    let mut frames = Vec::with_capacity(frame_data_list.len() as usize);
                    for i in 0..frame_data_list.len() {
                        frames.push(frame_from_reader(frame_data_list.get(i))?);
                    }
                    Ok(frames)
                })
                .await
        })
    }

    /// Writes frames by sending Cap'n Proto `ConFrameData` to the server.
    pub fn write_frames(&self, frames: &[ConFrame]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.runtime.block_on(async {
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async {
                    let service = connect_service(&self.endpoint).await?;

                    let mut request = service.write_frames_request();
                    {
                        let mut wr = request.get().init_req();
                        set_compatibility_stamp(wr.reborrow().init_compatibility());
                        let mut list = wr.reborrow().init_frames(frames.len() as u32);
                        for (i, frame) in frames.iter().enumerate() {
                            fill_frame_builder(list.reborrow().get(i as u32), frame)
                                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                        }
                    }
                    let response = request.send().promise.await?;
                    let result = response.get()?.get_result()?;
                    validate_compatibility_stamp(result.get_compatibility()?)?;
                    let bytes = result.get_file_contents()?;
                    Ok(bytes.to_vec())
                })
                .await
        })
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::*;
    use std::time::Duration;

    fn bad_stamp_parse(spec: &str) -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async {
                    let service = connect_service(&Endpoint::parse(spec)?).await?;
                    let mut request = service.parse_frames_request();
                    {
                        let mut parse_request = request.get().init_req();
                        let mut stamp = parse_request.reborrow().init_compatibility();
                        stamp.set_abi_major(99);
                        stamp.set_abi_minor(0);
                        stamp.set_abi_layout_revision(0);
                        stamp.set_con_spec_version(0);
                        parse_request.set_file_contents(b"not used");
                    }
                    request.send().promise.await?;
                    Ok(())
                })
                .await
        })
    }

    fn short_cell_write(spec: &str) -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async {
                    let service = connect_service(&Endpoint::parse(spec)?).await?;
                    let mut request = service.write_frames_request();
                    {
                        let mut wr = request.get().init_req();
                        set_compatibility_stamp(wr.reborrow().init_compatibility());
                        let mut frame = wr.reborrow().init_frames(1).get(0);
                        frame.reborrow().init_cell(1).set(0, 1.0);
                        frame.init_angles(3);
                    }
                    request.send().promise.await?;
                    Ok(())
                })
                .await
        })
    }

    fn bad_stamp_write(spec: &str) -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async {
                    let service = connect_service(&Endpoint::parse(spec)?).await?;
                    let mut request = service.write_frames_request();
                    {
                        let mut wr = request.get().init_req();
                        let mut stamp = wr.reborrow().init_compatibility();
                        stamp.set_abi_major(99);
                        stamp.set_abi_minor(0);
                        stamp.set_abi_layout_revision(0);
                        stamp.set_con_spec_version(0);
                        wr.init_frames(0);
                    }
                    request.send().promise.await?;
                    Ok(())
                })
                .await
        })
    }

    const MINIMAL: &str = include_str!("../../resources/conformance/valid/v2_minimal.con");

    #[test]
    fn unix_socket_parse_roundtrip() {
        let sock = std::env::temp_dir().join(format!("readcon-rpc-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&sock);
        let spec = format!("unix:{}", sock.display());
        let spec_server = spec.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            let local = tokio::task::LocalSet::new();
            let _ = rt.block_on(local.run_until(crate::rpc::server::start_server(&spec_server)));
        });
        let started = std::time::Instant::now();
        while !sock.exists() {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "unix server did not bind {sock:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let client = RpcClient::new(&spec).expect("client");
        let frames = client.parse_bytes(MINIMAL.as_bytes()).expect("parse");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].atom_data.len(), 1);
        let written = client.write_frames(&frames).expect("write");
        let again = client.parse_bytes(&written).expect("parse written");
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].atom_data.len(), 1);
        assert!(client.parse_bytes(&[0xff, 0xfe]).is_err());
        assert!(client.parse_bytes(b"not a con frame").is_err());
        let rejected = bad_stamp_parse(&spec);
        assert!(rejected.is_err(), "a breaking ABI major must fail the parse");
        let rejected = short_cell_write(&spec);
        assert!(rejected.is_err(), "a short cell must fail the write");
        let rejected = bad_stamp_write(&spec);
        assert!(rejected.is_err(), "a breaking ABI major must fail the write");
        let mut bad_counts = frames[0].clone();
        bad_counts.header.natms_per_type = vec![99];
        assert!(
            client.write_frames(&[bad_counts]).is_err(),
            "a frame whose type count does not match its atoms must fail"
        );
        let _ = std::fs::remove_file(&sock);
    }

    #[test]
    fn tcp_parse_file_and_a_missing_path() {
        let port = 20_000 + (std::process::id() % 10_000) as u16;
        let spec = format!("127.0.0.1:{port}");
        let spec_server = spec.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            let local = tokio::task::LocalSet::new();
            let _ = rt.block_on(local.run_until(crate::rpc::server::start_server(&spec_server)));
        });
        let started = std::time::Instant::now();
        // An empty spec fails in the parser, a closed port builds a client
        // whose parse fails, and the live port then parses.
        let mut phase = 0u8;
        let client = loop {
            let attempt = match phase {
                0 => {
                    phase = 1;
                    ""
                }
                1 => {
                    phase = 2;
                    "127.0.0.1:1"
                }
                _ => spec.as_str(),
            };
            if let Ok(client) = RpcClient::new(attempt) {
                if client.parse_bytes(MINIMAL.as_bytes()).is_ok() {
                    break client;
                }
            }
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "tcp server did not accept connections"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("one.con");
        std::fs::write(&path, MINIMAL).unwrap();
        let frames = client.parse_file(&path).expect("parse file");
        assert_eq!(frames.len(), 1);
        assert!(client.parse_file(dir.path().join("missing.con").as_path()).is_err());
        let refused = RpcClient::new("127.0.0.1:1").expect("endpoint");
        let refused_err = refused.parse_bytes(MINIMAL.as_bytes()).unwrap_err();
        assert!(
            refused_err.to_string().to_lowercase().contains("refused"),
            "{refused_err}"
        );
        let bad = client.parse_bytes(&[0xff, 0xfe]).unwrap_err();
        assert!(
            !bad.to_string().to_lowercase().contains("refused"),
            "{bad}"
        );
    }
}
