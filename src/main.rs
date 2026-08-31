use std::net::SocketAddr;
use std::path::PathBuf;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use http_body_util::Full;

async fn handle_request(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path();
    let method = req.method();

    // -------------------------------------------------------------
    // 1. EVALUAR RUTAS DE API (/api/...)
    // -------------------------------------------------------------
    if path.starts_with("/api/") {
        return Ok(handle_api_routes(method, path).await);
    }

    // -------------------------------------------------------------
    // 2. BUSCAR ARCHIVOS ESTÁTICOS REALES (.js, .css, .png, etc.)
    // -------------------------------------------------------------
    let requested_path = path.trim_start_matches('/');
    let file_path = PathBuf::from("dist").join(requested_path);

    if file_path.is_file() {
        if let Ok(contents) = read_file_to_bytes(&file_path).await {
            let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();
            return Ok(create_response(StatusCode::OK, mime_type.as_ref(), contents));
        }
    }

    // -------------------------------------------------------------
    // 3. FALLBACK DE LA SPA (Redirigir todo lo demás a index.html)
    // -------------------------------------------------------------
    let index_path = PathBuf::from("dist/index.html");
    if let Ok(index_contents) = read_file_to_bytes(&index_path).await {
        return Ok(create_response(StatusCode::OK, "text/html", index_contents));
    }

    // Si dist/index.html no existe
    Ok(create_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "text/plain",
        Bytes::from("Error: dist/index.html no encontrado."),
    ))
}

// Sub-manejador exclusivo para endpoints de la API
async fn handle_api_routes(method: &Method, path: &str) -> Response<Full<Bytes>> {
    match (method, path) {
        // GET /api/usuarios
        (&Method::GET, "/api/usuarios") => {
            let json_payload = r#"[{"id": 1, "nombre": "Alice"}, {"id": 2, "nombre": "Bob"}]"#;
            create_response(StatusCode::OK, "application/json", Bytes::from(json_payload))
        }

        // GET /api/status
        (&Method::GET, "/api/status") => {
            let json_payload = r#"{"status": "ok", "version": "1.0"}"#;
            create_response(StatusCode::OK, "application/json", Bytes::from(json_payload))
        }

        // Ruta de API no encontrada (Devuelve 404 JSON, NO el index.html)
        _ => {
            let error_payload = r#"{"error": "Endpoint de API no encontrado"}"#;
            create_response(StatusCode::NOT_FOUND, "application/json", Bytes::from(error_payload))
        }
    }
}

// Funciones auxiliares
async fn read_file_to_bytes(path: &PathBuf) -> Result<Bytes, std::io::Error> {
    let mut file = File::open(path).await?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await?;
    Ok(Bytes::from(contents))
}

fn create_response(status: StatusCode, content_type: &str, body: Bytes) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .body(Full::new(body))
        .unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("Servidor SPA + API corriendo en http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                eprintln!("Error en la conexión: {:?}", err);
            }
        });
    }
}