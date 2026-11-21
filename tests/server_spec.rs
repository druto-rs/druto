use std::{sync::Arc, time::Duration};

use druto::prelude::*;
use futures_util::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct UserPayload {
    name: String,
    role: String,
}

#[tokio::test]
async fn test_in_memory_test_server_basic_get() {
    let mut router = Router::new();
    router.get(
        "/hello",
        from_handler(|ctx| {
            Box::pin(async move {
                ctx.response.text("Hello from Druto Framework!");
                Ok(())
            })
        }),
    );

    let client = TestServer::new(router).unwrap();
    let response = client.get("/hello").send().await.unwrap();

    response.assert_ok();
    response.assert_text("Hello from Druto Framework!");
}

#[tokio::test]
async fn test_in_memory_test_server_json_post() {
    let mut router = Router::new();
    router.post(
        "/users",
        from_handler(|ctx| {
            Box::pin(async move {
                let user: UserPayload = serde_json::from_slice(ctx.request.body()).unwrap();
                assert_eq!(user.name, "Druto User");

                let reply = UserPayload {
                    name: user.name,
                    role: "administrator".to_string(),
                };
                let _ = ctx.response.json(&reply);
                Ok(())
            })
        }),
    );

    let client = TestServer::new(router).unwrap();
    let payload = UserPayload {
        name: "Druto User".to_string(),
        role: "guest".to_string(),
    };

    let response = client
        .post("/users")
        .json(&payload)
        .unwrap()
        .send()
        .await
        .unwrap();
    response.assert_ok();

    let returned_user: UserPayload = response.json().unwrap();
    assert_eq!(returned_user.role, "administrator");
}

#[tokio::test]
async fn test_in_memory_test_server_route_parameters() {
    let mut router = Router::new();
    router.get(
        "/users/:id",
        from_handler(|ctx| {
            Box::pin(async move {
                let user_id = ctx.request.params().get("id").unwrap().to_string();
                ctx.response.text(format!("User Profile: {user_id}"));
                Ok(())
            })
        }),
    );

    let client = TestServer::new(router).unwrap();
    let response = client.get("/users/42").send().await.unwrap();

    response.assert_ok();
    response.assert_text("User Profile: 42");
}

#[tokio::test]
async fn test_in_memory_test_server_query_parameters() {
    let mut router = Router::new();
    router.get(
        "/search",
        from_handler(|ctx| {
            Box::pin(async move {
                let query_str = ctx.request.query().unwrap().to_string();
                ctx.response.text(format!("Query: {query_str}"));
                Ok(())
            })
        }),
    );

    let client = TestServer::new(router).unwrap();
    let response = client
        .get("/search")
        .query("page=1&sort=desc")
        .send()
        .await
        .unwrap();

    response.assert_ok();
    response.assert_text("Query: page=1&sort=desc");
}

#[tokio::test]
async fn test_serve_stream_with_raw_http1_payload() {
    let mut router = Router::new();
    router.get(
        "/ping",
        from_handler(|ctx| {
            Box::pin(async move {
                ctx.response.text("pong");
                Ok(())
            })
        }),
    );

    let (client, server) = create_in_memory_duplex();

    let server_task = tokio::spawn(async move { Server::serve_stream(server, router).await });

    let mut client_socket = client;
    client_socket
        .write_all(b"GET /ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    client_socket.close().await.unwrap();

    let result = server_task.await.unwrap();
    assert!(result.is_ok());

    let mut response_buf = Vec::new();
    client_socket.read_to_end(&mut response_buf).await.unwrap();
    let response_str = String::from_utf8_lossy(&response_buf);

    assert!(response_str.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response_str.contains("connection: close\r\n"));
    assert!(response_str.ends_with("pong"));
}

#[tokio::test]
async fn test_live_tcp_listener_and_graceful_shutdown() {
    // 1. Bind to ephemeral port 127.0.0.1:0
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let bound_addr = listener.local_addr().unwrap();
    drop(listener); // Free for Server::bind to take it

    let mut router = Router::new();
    router.get(
        "/status",
        from_handler(|ctx| {
            Box::pin(async move {
                ctx.response.text("Engine Operational");
                Ok(())
            })
        }),
    );

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server = Server::bind(bound_addr.to_string())
        .tcp_nodelay(true)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

    let server_task = tokio::spawn(async move { server.serve(router).await });

    // Wait for server to bind
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 2. Perform real TCP connection and HTTP GET
    let mut tcp_stream = tokio::net::TcpStream::connect(bound_addr).await.unwrap();
    let request = b"GET /status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut tcp_stream, request)
        .await
        .unwrap();

    let mut response_bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut tcp_stream, &mut response_bytes)
        .await
        .unwrap();
    let response_str = String::from_utf8_lossy(&response_bytes);

    assert!(response_str.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response_str.ends_with("Engine Operational"));

    // 3. Trigger graceful shutdown
    let _ = shutdown_tx.send(());

    // Connect dummy stream to unblock accept if pending
    let _ = tokio::net::TcpStream::connect(bound_addr).await;

    let server_result = server_task.await.unwrap();
    assert!(server_result.is_ok());
}

#[test]
fn test_server_from_env_defaults() {
    unsafe {
        std::env::remove_var("HOST");
        std::env::remove_var("PORT");
    }
    let server = Server::from_env();
    assert_eq!(server.tcp_addr(), Some("127.0.0.1:5555"));
}

#[tokio::test]
async fn test_druto_ignitor_lifecycle() {
    let mut router = Router::new();
    router.get(
        "/ping",
        from_handler(|ctx| {
            Box::pin(async move {
                ctx.response.text("pong");
                Ok(())
            })
        }),
    );

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_task = tokio::spawn(async move {
        Ignitor::new()
            .routes(router)
            .bind("127.0.0.1:0")
            .banner(false)
            .env(false)
            .tracing(false)
            .with_graceful_shutdown(async move {
                let () = shutdown_rx.await.unwrap();
            })
            .start()
            .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(shutdown_tx.send(()).is_ok());

    let result = server_task.await.unwrap();
    assert!(result.is_ok());
}

// Simple in-memory duplex stream for testing serve_stream without ports
struct InMemBufferInner {
    data: Vec<u8>,
    cursor: usize,
    closed: bool,
    waker: Option<std::task::Waker>,
}

#[derive(Clone)]
struct InMemBuffer(Arc<std::sync::RwLock<InMemBufferInner>>);

struct InMemDuplex {
    reader: InMemBuffer,
    writer: InMemBuffer,
}

fn create_in_memory_duplex() -> (InMemDuplex, InMemDuplex) {
    let client_to_server = InMemBuffer(Arc::new(std::sync::RwLock::new(InMemBufferInner {
        data: Vec::new(),
        cursor: 0,
        closed: false,
        waker: None,
    })));
    let server_to_client = InMemBuffer(Arc::new(std::sync::RwLock::new(InMemBufferInner {
        data: Vec::new(),
        cursor: 0,
        closed: false,
        waker: None,
    })));

    let client = InMemDuplex {
        reader: server_to_client.clone(),
        writer: client_to_server.clone(),
    };
    let server = InMemDuplex {
        reader: client_to_server,
        writer: server_to_client,
    };
    (client, server)
}

impl futures_io::AsyncRead for InMemDuplex {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut inner = self.reader.0.write().unwrap();
        if inner.cursor < inner.data.len() {
            let available = inner.data.len() - inner.cursor;
            let to_read = std::cmp::min(buf.len(), available);
            buf[..to_read].copy_from_slice(&inner.data[inner.cursor..inner.cursor + to_read]);
            inner.cursor += to_read;
            std::task::Poll::Ready(Ok(to_read))
        } else if inner.closed {
            std::task::Poll::Ready(Ok(0))
        } else {
            inner.waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}

impl futures_io::AsyncWrite for InMemDuplex {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let _ = cx;
        let mut inner = self.writer.0.write().unwrap();
        inner.data.extend_from_slice(buf);
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let _ = cx;
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let _ = cx;
        let mut inner = self.writer.0.write().unwrap();
        inner.closed = true;
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
        std::task::Poll::Ready(Ok(()))
    }
}

impl Drop for InMemDuplex {
    fn drop(&mut self) {
        let mut inner = self.writer.0.write().unwrap();
        inner.closed = true;
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }
}

#[cfg(feature = "full")]
#[tokio::test]
async fn test_facade_reexports_and_middleware_wiring() {
    let mut router = Router::new();

    let cors = CorsMiddleware::new(
        CorsConfig::builder()
            .allow_origin(OriginPolicy::Any)
            .build(),
    );
    router.middleware(cors);

    let shield = ShieldMiddleware::new(
        ShieldConfig::builder()
            .preset(SecurityPreset::SecureDefaults)
            .build(),
    );
    router.middleware(shield);

    let limiter = LimiterMiddleware::builder().build();
    router.middleware(limiter);

    let bodyparser = BodyParserMiddleware::default_config();
    router.middleware(bodyparser);

    let session_mgr = Arc::new(SessionManager::new(
        SessionConfig::builder().build(),
        Arc::new(druto::session::MemorySessionStore::new()),
    ));
    router.middleware(SessionMiddleware::new(session_mgr));

    router.get(
        "/facade-check",
        from_handler(|ctx| {
            Box::pin(async move {
                ctx.response.text("Facade OK");
                Ok(())
            })
        }),
    );

    let client = TestServer::new(router).unwrap();
    let response = client.get("/facade-check").send().await.unwrap();

    response.assert_ok();
    response.assert_text("Facade OK");
}

#[cfg(feature = "full")]
#[test]
fn test_facade_submodule_namespaces() {
    use druto::{
        bodyparser, cors, crypto, env, events, hash, ignitor, limiter, redis, session, shield,
        string, validation,
    };

    assert_eq!(string::to_snake_case("UserProfile"), "user_profile");
    assert_eq!(string::pluralize("category"), "categories");

    let cors_config = cors::CorsConfig::default();
    drop(cors_config);

    let limiter_config = limiter::LimiterConfig::default();
    drop(limiter_config);

    let shield_config = shield::ShieldConfig::default();
    drop(shield_config);

    let bodyparser_mw = bodyparser::BodyParserMiddleware::default_config();
    drop(bodyparser_mw);

    let session_config = session::SessionConfig::default();
    drop(session_config);

    let redis_config = redis::RedisConfig::default();
    drop(redis_config);

    let keyring = crypto::KeyRing::new(crypto::Key::generate().unwrap());
    let encrypter = crypto::Encrypter::new(keyring);
    drop(encrypter);

    let loader = env::EnvLoader::new();
    drop(loader);

    let dispatcher = events::Dispatcher::new();
    drop(dispatcher);

    let hash_mgr = hash::HashManager::builder().build();
    drop(hash_mgr);

    let ignitor_builder = ignitor::Ignitor::new();
    drop(ignitor_builder);

    let schema = validation::schema::object();
    drop(schema);
}
