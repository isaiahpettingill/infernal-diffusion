//! Small localhost viewer and fight sandbox for generated packages.
use crate::proto::Monster;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const INDEX: &str = include_str!("../web/index.html");
const SCRIPT: &str = include_str!("../web/app.js");
const STYLE: &str = include_str!("../web/style.css");

fn response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}

fn error(stream: &mut TcpStream, status: &str, message: &str) {
    response(
        stream,
        status,
        "application/json; charset=utf-8",
        json!({"error": message}).to_string().as_bytes(),
    );
}

fn read_request(stream: &mut TcpStream) -> Result<(String, String, Vec<u8>), String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(15)))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let count = stream.read(&mut chunk).map_err(|e| e.to_string())?;
        if count == 0 || bytes.len() + count > 16_384 {
            return Err("request headers too large or incomplete".into());
        }
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(i) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let header = std::str::from_utf8(&bytes[..header_end]).map_err(|e| e.to_string())?;
    let first = header.lines().next().ok_or("missing request line")?;
    let mut parts = first.split_whitespace();
    let method = parts.next().ok_or("missing method")?.to_string();
    let path = parts.next().ok_or("missing path")?.to_string();
    let length = header
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse::<usize>())
        })
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or(0);
    if length > 8_192 {
        return Err("request body too large".into());
    }
    let mut body = bytes[header_end..].to_vec();
    while body.len() < length {
        let count = stream.read(&mut chunk).map_err(|e| e.to_string())?;
        if count == 0 {
            return Err("incomplete request body".into());
        }
        body.extend_from_slice(&chunk[..count]);
    }
    body.truncate(length);
    Ok((method, path, body))
}

fn monster_json(monster: &Monster, asset_base: &str) -> Value {
    let sprites = monster
        .sprites
        .as_ref()
        .expect("validated monster has sprites");
    let gameplay = monster
        .gameplay
        .as_ref()
        .expect("validated monster has gameplay");
    let size = monster.size.as_ref();
    json!({
        "id": monster.id,
        "name": monster.display_name,
        "description": monster.description,
        "tags": monster.tags,
        "sprites": {
            "url": format!("{asset_base}/sprites.png"),
            "frameWidth": sprites.frame_width,
            "frameHeight": sprites.frame_height,
            "columns": sprites.columns,
            "stride": sprites.direction_stride,
            "angles": sprites.direction_angles_deg,
        },
        "projectileAtlas": format!("{asset_base}/projectiles.png"),
        "projectiles": monster.projectiles.iter().map(|p| json!({
            "id": p.id, "kind": p.kind, "frameWidth": p.frame_width,
            "frameHeight": p.frame_height, "columns": p.columns,
            "firstFrame": p.first_frame, "frameCount": p.frame_count,
            "frameDuration": p.frame_duration_ms,
        })).collect::<Vec<_>>(),
        "animations": monster.animations.iter().map(|a| json!({
            "id": a.id, "state": a.semantic_state, "loop": a.looped,
            "frames": a.frames.iter().map(|f| json!({
                "id": f.sprite_frame_id, "duration": f.duration_ms,
                "dx": f.root_dx, "dy": f.root_dy,
            })).collect::<Vec<_>>(),
            "events": a.events.iter().map(|e| json!({
                "time": e.time_ms, "kind": e.kind, "reference": e.reference_id,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "attacks": monster.attacks.iter().map(|a| json!({
            "id": a.id, "delivery": a.delivery, "element": a.element,
            "damage": a.damage, "range": a.range, "cooldown": a.cooldown_ms,
            "telegraph": a.telegraph_ms, "animation": a.animation_id,
            "projectile": a.projectile_id,
            "steps": a.steps.iter().map(|step| json!({
                "kind": step.primitive, "time": step.at_ms, "value": step.value,
                "target": step.target,
                "speed": step.speed, "radius": step.radius,
                "duration": step.duration_ms, "count": step.count,
                "spread": step.spread_deg,
            })).collect::<Vec<_>>(),
            "spawn": a.spawn.as_ref().map(|s| json!({
                "monsterId": s.monster_id, "count": s.count, "maxActive": s.max_active,
            })),
        })).collect::<Vec<_>>(),
        "movementModes": monster.movement_modes.iter().map(|m| json!({
            "id": m.id, "speed": m.speed, "animation": m.animation_id,
        })).collect::<Vec<_>>(),
        "colliders": monster.colliders.iter().filter(|c| c.hurtbox).map(|c| json!({
            "node": c.node_id,
            "x": c.x, "y": c.y, "radius": c.radius,
            "views": c.views.iter().map(|v| json!({
                "direction": v.direction_index, "x": v.x, "y": v.y, "radius": v.radius,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "health": gameplay.health,
        "defense": gameplay.defense,
        "healthParts": {
            "base": 20, "size": gameplay.health_size_bonus,
            "armor": gameplay.health_armor_bonus, "magic": gameplay.health_magic_bonus,
        },
        "size": size.map(|s| json!({
            "normalized": s.normalized_size, "class": s.size_class,
            "visibleWidth": s.visible_width_px, "visibleHeight": s.visible_height_px,
            "anchorX": s.anchor_x_px, "anchorY": s.anchor_y_px,
        })),
    })
}

fn package_json(monster: &Monster, key: &str, directory: &Path) -> Result<Value, crate::Error> {
    package_json_inner(monster, key, directory, 0)
}

fn package_json_inner(
    monster: &Monster,
    key: &str,
    directory: &Path,
    depth: u8,
) -> Result<Value, crate::Error> {
    if depth > 4 {
        return Err(crate::Error::InvalidMonster(
            "companion nesting exceeds four levels".into(),
        ));
    }
    let mut value = monster_json(monster, &format!("/asset/{key}"));
    let mut companions = serde_json::Map::new();
    if let Some(mount) = &monster.mount {
        for (role, path) in [
            ("rider", &mount.rider_package),
            ("mount", &mount.mount_package),
        ] {
            let child = crate::load_package(&directory.join(path))?;
            companions.insert(
                role.into(),
                package_json_inner(
                    &child,
                    &format!("{key}/{path}"),
                    &directory.join(path),
                    depth + 1,
                )?,
            );
        }
        value["mount"] = json!({"survivor": mount.survivor});
    }
    if let Some(spawn) = monster
        .attacks
        .iter()
        .find_map(|attack| attack.spawn.as_ref())
    {
        let child = crate::load_package(&directory.join(&spawn.package_path))?;
        companions.insert(
            "minion".into(),
            package_json_inner(
                &child,
                &format!("{key}/{}", spawn.package_path),
                &directory.join(&spawn.package_path),
                depth + 1,
            )?,
        );
    }
    value["companions"] = Value::Object(companions);
    Ok(value)
}

fn package_dir(root: &Path, prompt: &str, seed: u64) -> (String, PathBuf) {
    let mut digest = Sha256::new();
    digest.update(env!("CARGO_PKG_VERSION").as_bytes());
    digest.update(crate::render3d::recipe_fingerprint().as_bytes());
    digest.update(b"morphology.seeded_mystery@1");
    for recipe in crate::recipes::default_registry()
        .versions("HUMANOID")
        .into_iter()
        .chain(crate::recipes::mesh_registry().versions("HUMANOID"))
    {
        digest.update(recipe.as_bytes());
        digest.update([0]);
    }
    digest.update(prompt.as_bytes());
    digest.update(seed.to_le_bytes());
    let key = digest.finalize()[..10]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let path = root.join(&key);
    (key, path)
}

fn handle(mut stream: TcpStream, root: Arc<PathBuf>, generation_lock: Arc<Mutex<()>>) {
    let (method, path, body) = match read_request(&mut stream) {
        Ok(request) => request,
        Err(message) => return error(&mut stream, "400 Bad Request", &message),
    };
    match (method.as_str(), path.as_str()) {
        ("GET", "/") => response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            INDEX.as_bytes(),
        ),
        ("GET", "/app.js") => response(
            &mut stream,
            "200 OK",
            "text/javascript; charset=utf-8",
            SCRIPT.as_bytes(),
        ),
        ("GET", "/style.css") => response(
            &mut stream,
            "200 OK",
            "text/css; charset=utf-8",
            STYLE.as_bytes(),
        ),
        ("POST", "/api/generate") => {
            let input: Value = match serde_json::from_slice(&body) {
                Ok(value) => value,
                Err(_) => return error(&mut stream, "400 Bad Request", "invalid JSON"),
            };
            let Some(prompt) = input.get("prompt").and_then(Value::as_str) else {
                return error(&mut stream, "400 Bad Request", "prompt is required");
            };
            let prompt = prompt.trim();
            if prompt.is_empty() || prompt.len() > 4096 {
                return error(
                    &mut stream,
                    "400 Bad Request",
                    "prompt must be 1–4096 bytes",
                );
            }
            let Some(seed) = input.get("seed").and_then(Value::as_u64) else {
                return error(
                    &mut stream,
                    "400 Bad Request",
                    "seed must be an unsigned integer",
                );
            };
            let (key, directory) = package_dir(&root, prompt, seed);
            let monster = {
                let _guard = generation_lock
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if directory.join("monster.pb").exists() {
                    crate::load_package(&directory)
                } else {
                    crate::generate(prompt, seed, &directory, None)
                }
            };
            match monster.and_then(|monster| package_json(&monster, &key, &directory)) {
                Ok(value) => response(
                    &mut stream,
                    "200 OK",
                    "application/json; charset=utf-8",
                    value.to_string().as_bytes(),
                ),
                Err(message) => error(
                    &mut stream,
                    "500 Internal Server Error",
                    &message.to_string(),
                ),
            }
        }
        _ if method == "GET" && path.starts_with("/asset/") => {
            let parts: Vec<_> = path.split('/').collect();
            if parts.len() < 4 {
                return error(&mut stream, "404 Not Found", "asset not found");
            }
            let segments = &parts[3..parts.len().saturating_sub(1)];
            let nested = segments.len() <= 8
                && segments.len() % 2 == 0
                && segments.as_chunks::<2>().0.iter().all(|pair| {
                    pair[0] == "companions" && ["rider", "mount", "minion"].contains(&pair[1])
                });
            let file = parts.last().copied().unwrap_or("");
            if !nested
                || parts[2].len() != 20
                || !parts[2].bytes().all(|b| b.is_ascii_hexdigit())
                || ![
                    "sprites.png",
                    "projectiles.png",
                    "emission.png",
                    "preview.png",
                ]
                .contains(&file)
            {
                return error(&mut stream, "404 Not Found", "asset not found");
            }
            let mut asset = root.join(parts[2]);
            for pair in segments.as_chunks::<2>().0 {
                asset = asset.join(pair[0]).join(pair[1]);
            }
            match std::fs::read(asset.join(file)) {
                Ok(data) => response(&mut stream, "200 OK", "image/png", &data),
                Err(_) => error(&mut stream, "404 Not Found", "asset not found"),
            }
        }
        _ => error(&mut stream, "404 Not Found", "route not found"),
    }
}

pub fn serve(port: u16, output_root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(output_root)?;
    let root = Arc::new(output_root.to_path_buf());
    let generation_lock = Arc::new(Mutex::new(()));
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    println!("Infernal Diffusion: http://localhost:{port} (listening on 0.0.0.0:{port})");
    for stream in listener.incoming() {
        let stream = stream?;
        let root = Arc::clone(&root);
        let generation_lock = Arc::clone(&generation_lock);
        std::thread::spawn(move || handle(stream, root, generation_lock));
    }
    Ok(())
}
