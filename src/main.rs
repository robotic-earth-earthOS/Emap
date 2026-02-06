use actix_web::{delete, get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::fs;
use display_info::DisplayInfo;

#[get("/")]
async fn index() -> impl Responder {
    let monitors = DisplayInfo::all().unwrap_or_default();
    if monitors.len() > 1 {
        HttpResponse::Ok()
            .content_type("text/html")
            .body(include_str!("../html/setup.html"))
    } else {
        HttpResponse::Ok()
            .content_type("text/html")
            .body(include_str!("../html/Emap.html"))
    }
}

#[get("/dashboard")]
async fn dashboard() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../html/dashboard.html"))
}

#[get("/projection")]
async fn projection() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../html/projection.html"))
}

#[get("/lib/babel.min.js")]
async fn babel() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/javascript")
        .body(include_str!("../html/lib/babel.min.js"))
}

#[get("/lib/tailwind.js")]
async fn tailwind() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/javascript")
        .body(include_str!("../html/lib/tailwind.js"))
}

#[get("/lib/react.min.js")]
async fn react() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/javascript")
        .body(include_str!("../html/lib/react.min.js"))
}

#[get("/lib/react-dom.min.js")]
async fn react_dom() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/javascript")
        .body(include_str!("../html/lib/react-dom.min.js"))
}

#[get("/robotic T M.png")]
async fn logo() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/png")
        .body(include_bytes!("../html/robotic T M.png") as &'static [u8])
}

// --- Database & API ---

struct AppState {
    db: Mutex<Connection>,
}

#[derive(Serialize, Deserialize)]
struct AssetMeta {
    id: String,
    name: String,
    mime_type: String,
}

#[derive(Serialize, Deserialize)]
struct AppConfig {
    control_panel_monitor_id: u32,
}

#[derive(Serialize)]
struct MonitorInfo {
    id: u32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    is_primary: bool,
}

#[get("/api/monitors")]
async fn get_monitors() -> impl Responder {
    let monitors = DisplayInfo::all().unwrap_or_default();
    let info: Vec<MonitorInfo> = monitors.into_iter().map(|m| MonitorInfo {
        id: m.id,
        x: m.x,
        y: m.y,
        width: m.width,
        height: m.height,
        is_primary: m.is_primary,
    }).collect();
    HttpResponse::Ok().json(info)
}

#[post("/api/config/monitor")]
async fn save_monitor_config(data: web::Data<AppState>, config: web::Json<AppConfig>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let config_str = serde_json::to_string(&*config).unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO system_data (key, value) VALUES (?1, ?2)",
        params!["monitor_config", config_str],
    ).unwrap();
    HttpResponse::Ok().finish()
}

#[get("/api/kv/{key}")]
async fn get_kv(data: web::Data<AppState>, key: web::Path<String>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let res: Result<String, _> = conn.query_row(
        "SELECT value FROM kv_store WHERE key = ?1",
        params![key.as_str()],
        |row| row.get(0),
    );

    match res {
        Ok(val) => HttpResponse::Ok().content_type("application/json").body(val),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[post("/api/kv/{key}")]
async fn save_kv(data: web::Data<AppState>, key: web::Path<String>, body: String) -> impl Responder {
    let conn = data.db.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
        params![key.as_str(), body],
    ).unwrap();
    HttpResponse::Ok().finish()
}

#[get("/api/assets")]
async fn list_assets(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, name, mime_type FROM assets").unwrap();
    let assets_iter = stmt.query_map([], |row| {
        Ok(AssetMeta {
            id: row.get(0)?,
            name: row.get(1)?,
            mime_type: row.get(2)?,
        })
    }).unwrap();

    let assets: Vec<AssetMeta> = assets_iter.map(|x| x.unwrap()).collect();
    HttpResponse::Ok().json(assets)
}

#[post("/api/asset/{id}")]
async fn save_asset(
    data: web::Data<AppState>, 
    id: web::Path<String>, 
    req: HttpRequest, 
    body: web::Bytes
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let name = req.headers().get("X-Asset-Name").and_then(|h| h.to_str().ok()).unwrap_or("unknown");
    let mime = req.headers().get("Content-Type").and_then(|h| h.to_str().ok()).unwrap_or("application/octet-stream");
    
    // Save to file system
    let file_path = format!("assets/{}", id);
    if let Err(_) = fs::write(&file_path, &body) {
        return HttpResponse::InternalServerError().body("Failed to save file");
    }

    conn.execute(
        "INSERT OR REPLACE INTO assets (id, name, mime_type) VALUES (?1, ?2, ?3)",
        params![id.as_str(), name, mime],
    ).unwrap();
    HttpResponse::Ok().finish()
}

#[get("/api/asset/{id}")]
async fn get_asset(data: web::Data<AppState>, id: web::Path<String>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let res: Result<String, _> = conn.query_row(
        "SELECT mime_type FROM assets WHERE id = ?1",
        params![id.as_str()],
        |row| row.get(0),
    );

    match res {
        Ok(mime) => {
            let file_path = format!("assets/{}", id);
            if let Ok(data) = fs::read(file_path) {
                HttpResponse::Ok().content_type(mime).body(data)
            } else {
                HttpResponse::NotFound().finish()
            }
        },
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[delete("/api/asset/{id}")]
async fn delete_asset(data: web::Data<AppState>, id: web::Path<String>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    conn.execute("DELETE FROM assets WHERE id = ?1", params![id.as_str()]).unwrap();
    
    let file_path = format!("assets/{}", id);
    let _ = fs::remove_file(file_path);

    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create assets directory
    fs::create_dir_all("assets")?;

    // Initialize Database
    let conn = Connection::open("emap.db").expect("Failed to open database");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kv_store (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    ).expect("Failed to create kv_store table");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY,
            name TEXT,
            mime_type TEXT
        )",
        [],
    ).expect("Failed to create assets table");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS system_data (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    ).expect("Failed to create system_data table");

    let app_state = web::Data::new(AppState {
        db: Mutex::new(conn),
    });

    println!("Starting server on http://127.0.0.1:8080");
    println!("Open http://127.0.0.1:8080 in your browser.");

    HttpServer::new(move || {
        App::new()
            // Increase upload limit to 1GB
            .app_data(web::PayloadConfig::new(1024 * 1024 * 1024))
            .app_data(app_state.clone())
            .service(index)
            .service(dashboard)
            .service(projection)
            .service(babel)
            .service(tailwind)
            .service(react)
            .service(react_dom)
            .service(logo)
            .service(get_monitors)
            .service(save_monitor_config)
            .service(get_kv)
            .service(save_kv)
            .service(list_assets)
            .service(save_asset)
            .service(get_asset)
            .service(delete_asset)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}