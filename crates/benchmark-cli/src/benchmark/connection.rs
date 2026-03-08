use bytes::BytesMut;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::cli::Args;
use crate::resp::{ExpectedResponse, consume_response, encode_resp_parts};
use crate::workload::{BenchKind, BenchRun};

use super::model::Shared;
use super::request::build_setup_command;

const SETUP_BATCH: usize = 64;

pub(crate) struct Connection {
    pub stream: TcpStream,
    pub parse_buf: BytesMut,
}

pub async fn maybe_warn_about_server_config(args: &Args) {
    match try_fetch_server_config(args).await {
        Ok(()) => {}
        Err(_) => eprintln!("WARNING: Could not fetch server CONFIG"),
    }
}

async fn try_fetch_server_config(args: &Args) -> Result<(), String> {
    let addr = format!("{}:{}", args.host, args.port);
    let mut stream = TcpStream::connect(&addr)
        .await
        .map_err(|err| format!("connect {addr}: {err}"))?;
    stream
        .set_nodelay(true)
        .map_err(|err| format!("set_nodelay: {err}"))?;

    let mut parse_buf = BytesMut::with_capacity(1024);
    authenticate_and_select(
        &mut stream,
        &mut parse_buf,
        args.user.as_deref(),
        args.password.as_deref(),
        args.dbnum,
    )
    .await?;

    let parts = [b"CONFIG".to_vec(), b"GET".to_vec(), b"save".to_vec()];
    let part_refs = parts.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let payload = encode_resp_parts(&part_refs);
    stream
        .write_all(&payload)
        .await
        .map_err(|err| format!("CONFIG write failed: {err}"))?;
    consume_response(&mut stream, &mut parse_buf, None, None, false).await
}

pub(crate) async fn open_connection(cfg: &Shared) -> Result<Connection, String> {
    let addr = format!("{}:{}", cfg.host, cfg.port);
    let mut stream = TcpStream::connect(&addr)
        .await
        .map_err(|err| format!("connect {addr}: {err}"))?;
    stream
        .set_nodelay(true)
        .map_err(|err| format!("set_nodelay: {err}"))?;

    let mut parse_buf = BytesMut::with_capacity(8192);
    authenticate_and_select(
        &mut stream,
        &mut parse_buf,
        cfg.user.as_deref(),
        cfg.password.as_deref(),
        cfg.run.dbnum,
    )
    .await?;

    Ok(Connection { stream, parse_buf })
}

pub(crate) async fn authenticate_and_select_for_idle(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    user: Option<&str>,
    password: Option<&str>,
    dbnum: u32,
) -> Result<(), String> {
    authenticate_and_select(stream, parse_buf, user, password, dbnum).await
}

async fn authenticate_and_select(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    user: Option<&str>,
    password: Option<&str>,
    dbnum: u32,
) -> Result<(), String> {
    if let Some(password) = password {
        let auth = match user {
            Some(user) => encode_resp_parts(&[b"AUTH", user.as_bytes(), password.as_bytes()]),
            None => encode_resp_parts(&[b"AUTH", password.as_bytes()]),
        };
        stream
            .write_all(&auth)
            .await
            .map_err(|err| format!("AUTH write failed: {err}"))?;
        consume_response(
            stream,
            parse_buf,
            Some(&ExpectedResponse::Simple("OK")),
            Some(b"+OK\r\n"),
            true,
        )
        .await?;
    }

    if dbnum != 0 {
        let db = dbnum.to_string();
        let select = encode_resp_parts(&[b"SELECT", db.as_bytes()]);
        stream
            .write_all(&select)
            .await
            .map_err(|err| format!("SELECT write failed: {err}"))?;
        consume_response(
            stream,
            parse_buf,
            Some(&ExpectedResponse::Simple("OK")),
            Some(b"+OK\r\n"),
            true,
        )
        .await?;
    }

    Ok(())
}

pub(crate) async fn setup_connection_state(
    connection: &mut Connection,
    run: &BenchRun,
    key_base: &[u8],
    value: &[u8],
) -> Result<(), String> {
    match run.kind {
        BenchKind::Get
        | BenchKind::Lpop
        | BenchKind::Rpop
        | BenchKind::Spop
        | BenchKind::ZpopMin
        | BenchKind::Lrange100
        | BenchKind::Lrange300
        | BenchKind::Lrange500
        | BenchKind::Lrange600 => {
            prime_keyspace(
                &mut connection.stream,
                &mut connection.parse_buf,
                run,
                key_base,
                value,
            )
            .await
        }
        _ => Ok(()),
    }
}

async fn prime_keyspace(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    run: &BenchRun,
    key_base: &[u8],
    value: &[u8],
) -> Result<(), String> {
    let keyspace = run.random_keyspace_len.unwrap_or(1);
    let mut payload = Vec::new();
    let mut pending = 0usize;

    for slot in 0..keyspace {
        if let Some(command) = build_setup_command(run.kind, key_base, slot, value) {
            payload.extend_from_slice(&command);
            pending += 1;
        }

        if pending == SETUP_BATCH {
            flush_setup_batch(stream, parse_buf, &payload, pending).await?;
            payload.clear();
            pending = 0;
        }
    }

    if pending > 0 {
        flush_setup_batch(stream, parse_buf, &payload, pending).await?;
    }

    Ok(())
}

async fn flush_setup_batch(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    payload: &[u8],
    pending: usize,
) -> Result<(), String> {
    stream
        .write_all(payload)
        .await
        .map_err(|err| format!("setup write failed: {err}"))?;
    for _ in 0..pending {
        consume_response(stream, parse_buf, None, None, false).await?;
    }
    Ok(())
}
