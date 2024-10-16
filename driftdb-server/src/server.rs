use anyhow::Result;
use axum::{
    body::BoxBody,
    error_handling::HandleError,
    extract::{ws::WebSocket, Query, State, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use driftdb::{Database, MessageFromDatabase, MessageToDatabase};
use hyper::{Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::Opts;

struct TypedWebSocket<Inbound: DeserializeOwned, Outbound: Serialize> {
    socket: WebSocket,
    _ph_inbound: std::marker::PhantomData<Inbound>,
    _ph_outbound: std::marker::PhantomData<Outbound>,
}

impl<Inbound: DeserializeOwned, Outbound: Serialize> TypedWebSocket<Inbound, Outbound> {
    pub fn new(socket: WebSocket) -> Self {
        Self {
            socket,
            _ph_inbound: std::marker::PhantomData,
            _ph_outbound: std::marker::PhantomData,
        }
    }

    pub async fn recv(&mut self) -> Result<Option<Inbound>> {
        let msg = self.socket.recv().await.transpose()?;
        loop {
            match &msg {
                Some(msg) => match msg {
                    axum::extract::ws::Message::Close(_) => {
                        return Ok(None);
                    }
                    axum::extract::ws::Message::Ping(_) => {
                        self.socket
                            .send(axum::extract::ws::Message::Pong(vec![]))
                            .await?;
                    }
                    axum::extract::ws::Message::Pong(_) => {}
                    axum::extract::ws::Message::Binary(_) => {
                        return Err(anyhow::anyhow!("Binary messages are not supported."));
                    }
                    axum::extract::ws::Message::Text(msg) => {
                        let msg = serde_json::from_str(&msg)?;
                        return Ok(Some(msg));
                    }
                },
                None => return Ok(None),
            }
        }
    }

    pub async fn send(&mut self, msg: Outbound) -> Result<()> {
        let msg = serde_json::to_string(&msg)?;
        self.socket
            .send(axum::extract::ws::Message::Text(msg))
            .await?;
        Ok(())
    }
}

async fn handle_socket(socket: WebSocket, database: Arc<Database>, debug: bool) {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
    let mut socket: TypedWebSocket<MessageToDatabase, MessageFromDatabase> =
        TypedWebSocket::new(socket);

    let callback = move |message: &MessageFromDatabase| {
        let result = sender.try_send(message.clone());

        if let Err(err) = result {
            tracing::error!(
                ?err,
                "Failed to send message to user, probably already closed."
            );
        }
    };

    let conn = if debug {
        database.connect_debug(callback)
    } else {
        database.connect(callback)
    };

    loop {
        tokio::select! {
            msg = receiver.recv() => {
                // We've received a message from the database; forward it to user.

                let msg = msg.expect("Receiver should never be dropped before socket is closed.");

                socket.send(msg).await.expect("Failed to send message to user.");
            }
            msg = socket.recv() => {
                // We've received a message from the client; forward it to the database.

                match msg {
                    Ok(Some(msg)) => {
                        if let Err(e) = conn.send_message(&msg) {
                            tracing::error!(?e, "Failed to send message to database.");

                            let _ = socket.send(MessageFromDatabase::Error {
                                message: format!("Failed to send message to database: {}", e),
                            }).await;
                        }
                    },
                    Ok(None) => {
                        // Client has closed the connection.
                        break;
                    }
                    Err(err) => {
                        tracing::error!(?err, "Failed to receive message from user.");

                        let _ = socket.send(MessageFromDatabase::Error {
                            message: format!("Failed to receive message from user: {}", err),
                        }).await;

                        break;
                    }
                };
            }
        }
    }
}

#[derive(Deserialize)]
struct ConnectionQuery {
    #[serde(default)]
    debug: bool,
}

async fn connection(
    ws: WebSocketUpgrade,
    State(database): State<Arc<Database>>,
    Query(query): Query<ConnectionQuery>,
) -> Response<BoxBody> {
    ws.on_upgrade(move |socket| handle_socket(socket, database, query.debug))
}

pub fn api_routes() -> Result<Router> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(AllowOrigin::any());

    let database = Database::new();

    Ok(Router::new()
        .route("/ws", get(connection))
        .layer(cors)
        .with_state(Arc::new(database)))
}

async fn handle_servedir_error(err: std::io::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Something went wrong: {}", err),
    )
}

pub async fn run_server(opts: &Opts) -> anyhow::Result<()> {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    let app = Router::new()
        .nest("/api/", api_routes()?)
        .nest_service(
            "/",
            HandleError::new(ServeDir::new("../driftdb-ui/build"), handle_servedir_error),
        )
        .layer(trace_layer);
    let addr = SocketAddr::new(opts.host, opts.port);

    tracing::info!(?addr, "Server is listening.");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Err(anyhow::anyhow!("Server exited."))
}
