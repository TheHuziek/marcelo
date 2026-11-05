
// Incluir el mismo código autogenerado
pub mod usuarios {
    tonic::include_proto!("usuarios");
}
use std::path::Path;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::error::Error;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use http_body_util::Full;
use serde_json::{json, Value};
//use tonic::transport::Channel;

use usuarios::usuario_service_client::UsuarioServiceClient;
use usuarios::UsuarioRequest;
use http_body_util::BodyExt;
use serde::Deserialize;
use hyper::body::Incoming;

#[derive(Deserialize, Debug)]
struct CrearUsuarioRequest {
    pub nombre: String,
    pub email: String,
    pub password: String,
}
#[derive(Deserialize, Debug)]
struct LoginRequest {
    pub username: String,
    pub password: String,
}
#[derive(Deserialize, Debug)]
struct PasswordRequest {
    pub jwt: String,
}
async fn handle_request(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
// 1. Descomponer la petición en sus partes y su cuerpo
    let (parts, body) = req.into_parts();
    
    // 2. Extraer el path desde 'parts' (sin prestamos pendientes de 'req')
    let path = parts.uri.path();
    let method = &parts.method;

    if path.starts_with("/api/") {
        // 3. Ahora puedes pasar 'body' libremente
        return Ok(handle_api_routes(method, path, body).await);
    }
    // -------------------------------------------------------------
    // 2. BUSCAR ARCHIVOS ESTÁTICOS REALES (.js, .css, .png, etc.)
    // -------------------------------------------------------------
    let requested_path = path.trim_start_matches('/');
    let file_path = PathBuf::from("frontend/").join(requested_path);

    if file_path.is_file() {
        if let Ok(contents) = read_file_to_bytes(&file_path).await {
            let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();
            return Ok(create_response(StatusCode::OK, mime_type.as_ref(), contents));
        }
    }

    // -------------------------------------------------------------
    // 3. FALLBACK DE LA SPA (Redirigir todo lo demás a index.html)
    // -------------------------------------------------------------
    let index_path = PathBuf::from("frontend/index.html");
    if let Ok(index_contents) = read_file_to_bytes(&index_path).await {
        return Ok(create_response(StatusCode::OK, "text/html", index_contents));
    }

    // Si var/marlin/html/index.html no existe
    Ok(create_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "text/plain",
        Bytes::from("Error: var/marlin/html/index.html no encontrado."),
    ))
}

// Sub-manejador exclusivo para endpoints de la API
async fn handle_api_routes(method: &Method, path: &str, req: hyper::body::Incoming) -> Response<Full<Bytes>> {
    let path_vec: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match (method, path_vec.as_slice()) {
        // POST /api/login
        (&Method::POST, &["api", "login"]) => {
            // 1. Extraer y leer todos los bytes del cuerpo de la petición
            let body_bytes = match leer_cuerpo(req).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    return create_response(
                        StatusCode::BAD_REQUEST,
                        "application/json",
                        Bytes::from(format!(r#"{{"error": "{}"}}"#, err)),
                    );
                }
            };

            // 2. Parsear/Deserializar el JSON recibido a la estructura Rust
            let login_request: LoginRequest = match serde_json::from_slice(&body_bytes) {
                Ok(datos) => datos,
                Err(_) => {
                    return create_response(
                        StatusCode::BAD_REQUEST,
                        "application/json",
                        Bytes::from(r#"{"error": "Formato JSON inválido o faltan campos obligatorios"}"#),
                    );
                }
            };


            // 1. Obtenemos el string que devuelve action_auth
            let auth_response_str = action_auth(login_request.username, login_request.password, "login".to_string())
                .await
                .unwrap_or_else(|_| r#"{"statusCode": 500}"#.to_string());
                    
            // 2. Deserializamos la cadena para convertirla en un objeto JSON real (Value::Object)
            let auth_result: serde_json::Value = serde_json::from_str(&auth_response_str)
                .unwrap_or_else(|_| json!({"statusCode": 500}));
                    
            // 3. Extraemos el statusCode numérico de forma segura
            let status_code = auth_result["statusCode"].as_i64().unwrap_or(500);
                    
            // 4. Evaluamos según el estado
            match status_code {
                200 => {
                    // Opcional: Extraer el body si necesitas devolver el token al cliente
                    let respuesta_json = r#"{"mensaje": "Login exitoso"}"#;
                    create_response(StatusCode::OK, "application/json", Bytes::from(respuesta_json))
                }
                401 => create_response(
                    StatusCode::UNAUTHORIZED,
                    "application/json",
                    Bytes::from(r#"{"error": "Credenciales inválidas"}"#),
                ),
                500 => create_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "application/json",
                    Bytes::from(r#"{"error": "Error interno del servidor"}"#),
                ),
                _ => {
                    println!("Resultado inesperado de autenticación: {:?}", auth_result);
                    create_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "application/json",
                        Bytes::from(r#"{"error": "Resultado inesperado de autenticación"}"#),
                    )
                }
            }
        }
        // GET /api/status
        (&Method::GET, &["api", "status"]) => {
            let json_payload = r#"{"status": "ok", "version": "1.0"}"#;
            create_response(StatusCode::OK, "application/json", Bytes::from(json_payload))
        }
        (&Method::GET, &["api", "passwords"]) => {
            // Aquí iría la lógica para obtener los passwords, por ejemplo:
            let json_payload = r#"{"passwords": ["pass1", "pass2", "pass3"]}"#;
            create_response(StatusCode::OK, "application/json", Bytes::from(json_payload))
        }
        (&Method::POST, &["api", "create_usuario"]) => {
            // 1. Extraer y leer todos los bytes del cuerpo de la petición
            let body_bytes = match leer_cuerpo(req).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    return create_response(
                        StatusCode::BAD_REQUEST,
                        "application/json",
                        Bytes::from(format!(r#"{{"error": "{}"}}"#, err)),
                    );
                }
            };

            // 2. Parsear/Deserializar el JSON recibido a la estructura Rust
            let nuevo_usuario: CrearUsuarioRequest = match serde_json::from_slice(&body_bytes) {
                Ok(datos) => datos,
                Err(_) => {
                    return create_response(
                        StatusCode::BAD_REQUEST,
                        "application/json",
                        Bytes::from(r#"{"error": "Formato JSON inválido o faltan campos obligatorios"}"#),
                    );
                }
            };

            // 3. (Aquí iría tu lógica de negocio, ej. guardar en Base de Datos o llamar gRPC)
            // action_auth(nuevo_usuario.nombre.clone(), nuevo_usuario.password.clone(), "register".to_string()).await.unwrap();
            // 4. Responder con código 201 Created y los datos creados
            let respuesta_json = format!(
                r#"{{"mensaje": "Usuario creado con éxito", "nombre": "{}"}}"#,
                nuevo_usuario.nombre
            );

            create_response(StatusCode::CREATED, "application/json", Bytes::from(respuesta_json))
        }
        // Ruta de API no encontrada (Devuelve 404 JSON, NO el index.html)
        _ => {
            let error_payload = r#"{"error": "Endpoint de API no encontrado"}"#;
            create_response(StatusCode::NOT_FOUND, "application/json", Bytes::from(error_payload))
        }
    }
}
// Función para leer todo el cuerpo entrante como Bytes
async fn leer_cuerpo(body: Incoming) -> Result<hyper::body::Bytes, String> {
    body.collect()
        .await
        .map(|collected| collected.to_bytes())
        .map_err(|e| format!("Error al leer el cuerpo de la petición: {}", e))
}
// async fn action_auth(username: String, password: String,action: String) -> Result<String, Box<dyn Error>> {
//     // Carga la configuración de AWS desde el entorno (IAM Role de la EC2, ~/.aws/credentials, etc.)
//     let config = aws_config::load_from_env().await;
//     let client = aws_sdk_lambda::Client::new(&config);

//     // Tu Lambda extrae los datos de `event.get('body')`, 
//      let lambda_event = json!({
//             "body": json!({
//                 "action":action,
//                 "username": username,
//                 "password": password
//             }).to_string() // El body suele venir como string en eventos de API Gateway
//         });
//     let payload_blob = Blob::new(lambda_event.to_string().into_bytes());

//     // Ejecutar la invocación
//     let response = client
//         .invoke()
//         .function_name("AuthLoginFunction") 
//         .payload(payload_blob)
//         .send()
//         .await?;
//     response.payload();
//     let result_string = if let Some(payload) = response.payload() {
//         String::from_utf8(payload.clone().into_inner())?
//     } else {
//         String::new()
//     };

//     Ok(result_string)
// }
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
    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
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