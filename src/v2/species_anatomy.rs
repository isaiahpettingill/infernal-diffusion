//! V2 species landmarks and bounded semantic anomalies. Coordinates remain in the
//! native rest rig, so IK, attachments and damage origins share the same anatomy.
use super::{has, index, landmark, move_branch, remove_where};
use crate::{anatomy::Body, parser::MonsterSpec};
#[path = "dinosaur_anatomy.rs"]
mod dinosaurs;

pub(super) fn refine(spec: &MonsterSpec, body: &mut Body, seed: u64, s: f32) {
    if spec.body_plan == "HUMANOID"
        && matches!(
            spec.affinity.as_str(),
            "ORC"
                | "TROLL"
                | "GRENDEL"
                | "CYCLOPEAN"
                | "MINOTAUR"
                | "ANIME_GIRL"
                | "HITLER"
                | "REAPER"
                | "ALIEN"
                | "ROBOT"
        )
    {
        spatial_humanoid(spec, body, s);
    }
    let default_material = match spec.affinity.as_str() {
        "ORC" => Some("V2_ORC_SKIN"),
        "TROLL" => Some("V2_TROLL_SKIN"),
        "WOLF" | "CERBERUS" | "FENRIR" => Some("V2_WOLF_FUR"),
        "LION" | "TIGER" => Some("FUR_GOLD"),
        "BEAR" => Some("FUR_BROWN"),
        "ELEPHANT" | "RHINO" => Some("V2_HIDE_GRAY"),
        _ => None,
    };
    if let Some(material) = default_material {
        if spec
            .materials
            .iter()
            .all(|m| matches!(m.as_str(), "FLESH" | "FUR" | "FUR_GOLD"))
        {
            for n in &mut body.nodes {
                if n.material.starts_with("FLESH") || n.material.starts_with("FUR") {
                    n.material = material.into();
                }
            }
        }
    }
    match spec.affinity.as_str() {
        "ORC" | "TROLL" | "CYCLOPEAN" | "MINOTAUR" => humanoid(spec, body, s),
        "ANIME_GIRL" => adult_manga(body, s),
        "CRAB" | "LOBSTER" => crustacean(spec, body, s, seed),
        _ => {}
    }
    if spec.body_plan == "QUADRUPED" {
        quadruped(spec, body, s);
    }
    if matches!(spec.affinity.as_str(), "HYDRA" | "CERBERUS") {
        many_heads(spec, body, s);
    }
    if matches!(spec.affinity.as_str(), "WORM" | "SAND_WORM") {
        worm_dimensions(spec, body, s);
    }
    if spec.affinity == "BEAR" {
        for h in 0..spec.heads.min(5) {
            if let Some(i) = index(body, &format!("neck_{h}")) {
                let n = body.nodes[i].clone();
                move_branch(body, i, n.x, n.y + 2.5 * s, n.z.unwrap_or(0.0));
            }
        }
        for n in &mut body.nodes {
            if matches!(n.kind.as_str(), "LIMB" | "SHIN" | "FOOT") {
                n.rx *= 1.3;
                n.rz = n.rz.map(|v| v * 1.25);
            }
        }
    }
    dinosaurs::refine(spec, body, s);
    landmarks(spec, body, s);
    fantasy(spec, body, seed, s);
    if has(spec, "AI_ANATOMY_ECHO") {
        anomaly(body, seed);
    }
    if has(spec, "ZOMBIFIED")
        && spec.limb_count == 0
        && spec.arm_count == 0
        && !has(spec, "EXPLICIT_TAILS")
    {
        if let Some(last) = body
            .nodes
            .iter()
            .rev()
            .find(|n| n.kind == "TAIL")
            .map(|n| n.id.clone())
        {
            remove_where(body, |n| n.id == last);
        }
    }
    if has(spec, "CHIBI") {
        chibi(body);
    }
}

fn humanoid(spec: &MonsterSpec, body: &mut Body, s: f32) {
    let troll = spec.affinity == "TROLL";
    let bull = spec.affinity == "MINOTAUR";
    for n in &mut body.nodes {
        match n.kind.as_str() {
            "TORSO" => {
                n.rx *= if troll { 1.4 } else { 1.12 };
                n.ry *= if troll { 1.13 } else { 1.03 };
                n.rz = Some(if troll { 8.4 * s } else { 6.9 * s });
            }
            "SHOULDER" => {
                n.rx *= 1.65;
                n.ry *= 1.4;
                n.rz = Some(2.2 * s);
            }
            "ARM" => {
                n.rx *= if troll { 1.65 } else { 1.35 };
                n.ry *= 1.2;
                n.rz = Some(n.rx * 0.88);
            }
            "HAND" => {
                n.rx *= if troll { 1.5 } else { 1.15 };
                n.ry *= if troll { 1.45 } else { 1.12 };
                n.rz = Some(n.rx * 0.6);
            }
            "LIMB" => {
                n.rx *= if troll { 1.3 } else { 1.18 };
                n.rz = Some(n.rx * 0.88);
            }
            "SHIN" => {
                n.rx *= 0.88;
                n.rz = Some(n.rx * 0.9);
            }
            "FOOT" => {
                n.rx *= 1.1;
                n.rz = Some(1.65 * s);
            }
            "NECK" => {
                n.rx *= 1.1;
                n.rz = Some(2.6 * s);
            }
            "HEAD" => {
                n.rx *= if troll {
                    1.22
                } else if bull {
                    1.0
                } else {
                    1.08
                };
                n.ry *= if troll { 1.30 } else { 1.15 };
                n.rz = Some(if troll { 5.2 * s } else { 4.6 * s });
            }
            _ => {}
        }
    }
    if let Some(t) = index(body, "torso") {
        let m = body.nodes[t].material.clone();
        // Pectoral and scapular masses remain attached to the chest, not floating
        // random lobes; a narrower abdominal silhouette is supplied by its mesh.
        for side in [-1.0, 1.0] {
            landmark(
                body,
                t,
                if side < 0.0 {
                    "v2_pectoral_far"
                } else {
                    "v2_pectoral_near"
                },
                "LOBE",
                [2.0 * s, -3.6 * s, side * 2.5 * s],
                [2.5 * s, 2.0 * s, 2.2 * s],
                &m,
            );
        }
        if troll {
            landmark(
                body,
                t,
                "v2_troll_trapezius",
                "LOBE",
                [-2.7 * s, -6.6 * s, 0.0],
                [3.7 * s, 3.1 * s, 4.4 * s],
                &m,
            );
        }
    }
    if troll {
        for i in 0..spec.heads.min(5) {
            if let Some(h) = index(body, &format!("head_{i}")) {
                let n = body.nodes[h].clone();
                move_branch(body, h, n.x + 3.6 * s, n.y + 2.6 * s, n.z.unwrap_or(0.0));
            }
        }
    }
}

fn adult_manga(body: &mut Body, s: f32) {
    // Replace legacy school uniform pieces only inside V2. Fully clothed adult
    // fashion: fitted dark jacket/top, long trousers, boots, mature proportions.
    remove_where(body, |n| {
        matches!(n.id.as_str(), "skirt" | "hair_cap" | "hair_lock")
    });
    for n in &mut body.nodes {
        match n.kind.as_str() {
            "HEAD" => {
                n.rx *= 0.82;
                n.ry *= 0.9;
                n.rz = Some(3.8 * s);
            }
            "TORSO" => {
                n.rx *= 1.14;
                n.rz = Some(4.4 * s);
                n.material = "CLOTH_NAVY".into();
            }
            "SHOULDER" | "ARM" => {
                n.material = "CLOTH_NAVY".into();
            }
            "HIP" | "LIMB" | "SHIN" => {
                n.material = "LEATHER_BLACK".into();
                n.ry *= 1.08;
            }
            "FOOT" => {
                n.material = "LEATHER_BLACK".into();
                n.ry *= 1.22;
            }
            _ => {}
        }
    }
    if let Some(t) = index(body, "torso") {
        for (i, z) in [-1.0, 1.0].into_iter().enumerate() {
            landmark(
                body,
                t,
                &format!("v2_adult_jacket_bodice_{i}"),
                "LOBE",
                [2.7 * s, -3.4 * s, z * 2.3 * s],
                [2.3 * s, 2.7 * s, 2.0 * s],
                "CLOTH_NAVY",
            );
        }
        landmark(
            body,
            t,
            "v2_adult_jacket_hem",
            "LOBE",
            [0.0, 3.7 * s, 0.0],
            [5.0 * s, 2.1 * s, 3.3 * s],
            "CLOTH_NAVY",
        );
    }
}

fn quadruped(spec: &MonsterSpec, body: &mut Body, s: f32) {
    let canid = matches!(
        spec.affinity.as_str(),
        "WOLF" | "DOG" | "JACKAL" | "CERBERUS" | "FENRIR" | "HYENA"
    );
    let cat = matches!(
        spec.affinity.as_str(),
        "LION" | "TIGER" | "CAT" | "PANTHER" | "LEOPARD" | "CHEETAH" | "SABERTOOTH_CAT"
    );
    let hoof = matches!(
        spec.affinity.as_str(),
        "HORSE"
            | "ZEBRA"
            | "GOAT"
            | "RAM"
            | "BULL"
            | "BUFFALO"
            | "MOOSE"
            | "GAZELLE"
            | "ANTELOPE"
            | "GIRAFFE"
    );
    let heavy = matches!(
        spec.affinity.as_str(),
        "BEAR" | "RHINO" | "ELEPHANT" | "MAMMOTH"
    );
    for n in &mut body.nodes {
        match n.kind.as_str() {
            "TORSO" if canid => {
                n.ry *= 0.9;
                n.rz = Some(3.5 * s);
            }
            "TORSO" if cat => {
                n.ry *= 0.93;
                n.rz = Some(4.4 * s);
            }
            "TORSO" if heavy => {
                n.rz = Some(n.ry * 0.93);
            }
            "HIP" | "LIMB" if canid || cat => {
                n.rx *= if canid { 0.82 } else { 1.05 };
                n.rz = Some(n.rx * 0.92);
            }
            "SHIN" if canid || cat || hoof => {
                n.rx *= if hoof { 0.62 } else { 0.7 };
                n.rz = Some(n.rx * 0.85);
            }
            "FOOT" if canid || cat => {
                n.rx *= 0.85;
                n.ry *= 0.85;
                n.rz = Some(1.65 * s);
            }
            "NECK" if canid || cat => {
                n.rx *= 0.84;
                n.ry *= 1.2;
                n.rz = Some(3.0 * s);
            }
            "HEAD" if canid => {
                n.rx *= 0.93;
                n.ry *= 1.15;
                n.rz = Some(2.75 * s);
            }
            "HEAD" if cat => {
                let lion = spec.affinity == "LION";
                n.rx *= if lion { 0.90 } else { 0.84 };
                n.ry *= if lion { 1.45 } else { 1.2 };
                n.rz = Some(if lion { 3.6 * s } else { 3.0 * s });
            }
            "TAIL" if canid => {
                n.ry *= 0.78;
                n.rz = Some(1.5 * s);
            }
            "TAIL" if cat => {
                n.rx *= 1.25;
                n.ry *= 0.38;
                n.rz = Some(0.65 * s);
            }
            _ => {}
        }
    }
    if hoof || spec.affinity == "BOAR" {
        let horse = matches!(spec.affinity.as_str(), "HORSE" | "ZEBRA");
        for h in 0..spec.heads.min(5) {
            if let Some(i) = index(body, &format!("neck_{h}")) {
                let n = body.nodes[i].clone();
                move_branch(
                    body,
                    i,
                    n.x + 1.5 * s,
                    n.y - if horse { 4.5 * s } else { 2.5 * s },
                    n.z.unwrap_or(0.0),
                );
                body.nodes[i].ry *= if horse { 1.9 } else { 1.2 };
            }
            if let Some(i) = index(body, &format!("head_{h}")) {
                let n = body.nodes[i].clone();
                move_branch(
                    body,
                    i,
                    n.x + 2.0 * s,
                    n.y - if horse { 3.5 * s } else { 1.0 * s },
                    n.z.unwrap_or(0.0),
                );
                body.nodes[i].ry *= 1.15;
            }
        }
    }
    if heavy {
        for h in 0..spec.heads.min(5) {
            if let Some(i) = index(body, &format!("neck_{h}")) {
                let n = body.nodes[i].clone();
                move_branch(body, i, n.x + 2.0 * s, n.y - 2.0 * s, n.z.unwrap_or(0.0));
            }
            if let Some(i) = index(body, &format!("head_{h}")) {
                let n = body.nodes[i].clone();
                move_branch(body, i, n.x + 3.0 * s, n.y - 3.0 * s, n.z.unwrap_or(0.0));
                if spec.affinity == "BEAR" {
                    body.nodes[i].rx = 5.4 * s;
                    body.nodes[i].ry = 3.8 * s;
                    body.nodes[i].rz = Some(4.6 * s);
                } else {
                    body.nodes[i].ry = 4.3 * s;
                    body.nodes[i].rz = Some(4.0 * s);
                }
            }
        }
        if let Some(t) = index(body, "torso") {
            body.nodes[t].ry *= 0.88;
        }
        if spec.affinity == "BEAR" {
            if let Some(t) = index(body, "tail") {
                body.nodes[t].rx = 1.8 * s;
                body.nodes[t].ry = 1.6 * s;
            }
        }
    }
    if canid || cat {
        for leg in 0..spec.limb_count.min(12) {
            let rear = leg >= 2;
            if let Some(k) = index(body, &format!("limb_{leg}")) {
                let n = body.nodes[k].clone();
                move_branch(
                    body,
                    k,
                    n.x + if rear { 2.2 * s } else { -s },
                    n.y,
                    n.z.unwrap_or(0.0),
                );
            }
            if let Some(k) = index(body, &format!("shin_{leg}")) {
                let n = body.nodes[k].clone();
                move_branch(
                    body,
                    k,
                    n.x + if rear { -2.6 * s } else { 0.5 * s },
                    n.y,
                    n.z.unwrap_or(0.0),
                );
            }
        }
        for i in 0..spec.heads.min(5) {
            if let Some(h) = index(body, &format!("head_{i}")) {
                let n = body.nodes[h].clone();
                move_branch(body, h, n.x, n.y - 2.0 * s, n.z.unwrap_or(0.0));
            }
        }
    }
    if spec.affinity == "LION" {
        if let Some(t) = index(body, "tail") {
            let n = body.nodes[t].clone();
            landmark(
                body,
                t,
                "v2_tail_tuft",
                "LOBE",
                [n.x - n.rx * 0.9, n.y, n.z.unwrap_or(0.0)],
                [1.8 * s, 1.35 * s, 1.25 * s],
                "FUR_DARK",
            );
        }
    }
}

/// A worm is a continuous low tube, not a serpent head on an oval abdomen.
/// Keep the original segment joints and their parent links for motion/attacks.
fn worm_dimensions(spec: &MonsterSpec, body: &mut Body, s: f32) {
    let sand = spec.affinity == "SAND_WORM";
    let radius = if sand { 3.6 } else { 2.45 } * s;
    for n in &mut body.nodes {
        match n.kind.as_str() {
            "TORSO" => {
                n.x = 0.0;
                n.y = 0.0;
                n.rx = 9.5 * s;
                n.ry = radius;
                n.rz = Some(radius);
            }
            "NECK" => {
                n.x = 8.5 * s;
                n.y = -0.15 * s;
                n.rx = 3.1 * s;
                n.ry = radius * 0.96;
                n.rz = Some(radius * 0.96);
                n.angle = 0.0;
            }
            "HEAD" => {
                n.x = 11.7 * s;
                n.y = -0.2 * s;
                n.rx = if sand { 4.1 } else { 3.6 } * s;
                n.ry = if sand { 4.1 } else { 3.7 } * s;
                n.rz = Some(n.ry);
                n.angle = 0.0;
            }
            "TAIL" => {
                if let Some(i) =
                    n.id.strip_prefix("tail_")
                        .and_then(|v| v.parse::<usize>().ok())
                {
                    let taper = (1.0 - i as f32 * if sand { 0.085 } else { 0.12 }).max(0.25);
                    n.x = (-9.2 - i as f32 * 3.55) * s;
                    n.y = (0.55 * (i as f32 * 0.65).sin()) * s;
                    n.rx = 2.55 * s;
                    n.ry = radius * taper;
                    n.rz = Some(radius * taper);
                    n.angle = 0.0;
                }
            }
            _ => {}
        }
    }
}

fn many_heads(spec: &MonsterSpec, body: &mut Body, s: f32) {
    for h in 0..spec.heads.min(5) {
        let side = h as f32 - (spec.heads.min(5) as f32 - 1.0) / 2.0;
        if let Some(neck) = index(body, &format!("neck_{h}")) {
            let y = if spec.affinity == "HYDRA" {
                -10.0 - side.abs() * 1.2
            } else {
                -3.8
            };
            move_branch(
                body,
                neck,
                if spec.affinity == "HYDRA" {
                    (5.5 - side.abs()) * s
                } else {
                    7.5 * s
                },
                y * s,
                side * 4.6 * s,
            );
            body.nodes[neck].rx = if spec.affinity == "HYDRA" {
                1.8 * s
            } else {
                2.4 * s
            };
            body.nodes[neck].rz = Some(body.nodes[neck].rx);
        }
        if let Some(head) = index(body, &format!("head_{h}")) {
            move_branch(
                body,
                head,
                (12.0 - side.abs() * 1.2) * s,
                if spec.affinity == "HYDRA" {
                    (-18.0 + side.abs() * 2.0) * s
                } else {
                    (-6.7 + side.abs()) * s
                },
                side * 5.6 * s,
            );
            body.nodes[head].rz = Some(2.5 * s);
        }
    }
}

fn landmarks(spec: &MonsterSpec, body: &mut Body, s: f32) {
    if spec.affinity == "RHINO" {
        if let Some(h) = index(body, "head_0") {
            let n = body.nodes[h].clone();
            remove_where(body, |n| n.kind == "HORN" && !n.id.starts_with("hybrid_"));
            let h = index(body, "head_0").unwrap();
            landmark(
                body,
                h,
                "v2_rhino_nose_horn",
                "HORN",
                [n.x + 3.9 * s, n.y - 2.3 * s, 0.0],
                [2.5 * s, 4.8 * s, 1.8 * s],
                "HORN",
            );
            landmark(
                body,
                h,
                "v2_rhino_rear_horn",
                "HORN",
                [n.x + 1.0 * s, n.y - 2.8 * s, 0.0],
                [1.8 * s, 2.5 * s, 1.2 * s],
                "HORN",
            );
        }
    }
    if matches!(
        spec.affinity.as_str(),
        "ELEPHANT" | "MAMMOTH" | "GROOTSLANG"
    ) {
        if let Some(h) = index(body, "head_0") {
            let head = body.nodes[h].clone();
            for i in 0..2 {
                if index(body, &format!("tusk_{i}")).is_none() {
                    landmark(
                        body,
                        h,
                        &format!("tusk_{i}"),
                        "TUSK",
                        [
                            head.x + 2.5 * s,
                            head.y + 1.5 * s,
                            if i == 0 { 2.8 * s } else { -2.8 * s },
                        ],
                        [4.5 * s, 2.3 * s, 0.7 * s],
                        "BONE",
                    );
                }
            }
            for i in 0..2 {
                if let Some(t) = index(body, &format!("tusk_{i}")) {
                    let n = &mut body.nodes[t];
                    n.x = head.x + 2.5 * s;
                    n.y = head.y + 1.5 * s;
                    n.z = Some(if i == 0 { 2.8 * s } else { -2.8 * s });
                    n.rx = if spec.affinity == "MAMMOTH" {
                        4.2 * s
                    } else {
                        4.5 * s
                    };
                    n.ry = if spec.affinity == "MAMMOTH" {
                        4.0 * s
                    } else {
                        2.3 * s
                    };
                    n.rz = Some(0.7 * s);
                    n.angle = 0.0;
                }
            }
            if let Some(t) = index(body, "trunk") {
                let n = &mut body.nodes[t];
                n.x = head.x + 3.9 * s;
                n.y = head.y + 6.4 * s;
                n.rx = 2.9 * s;
                n.ry = 10.5 * s;
                n.rz = Some(2.0 * s);
                n.angle = 0.0;
            }
        }
    }
    if matches!(spec.affinity.as_str(), "MINOTAUR" | "BULL" | "GOAT" | "RAM") {
        remove_where(body, |n| n.id == "horn_0");
        if let Some(h) = index(body, "head_0") {
            let n = body.nodes[h].clone();
            for (i, z) in [-1.0, 1.0].into_iter().enumerate() {
                landmark(
                    body,
                    h,
                    &format!("v2_bull_horn_{i}"),
                    "HORN",
                    [n.x - 1.3 * s, n.y - 2.6 * s, z * 3.3 * s],
                    [1.2 * s, 4.0 * s, 1.2 * s],
                    "HORN",
                );
            }
        }
    }
}

fn crustacean(spec: &MonsterSpec, body: &mut Body, s: f32, seed: u64) {
    let shell = spec
        .materials
        .iter()
        .find(|m| !matches!(m.as_str(), "CHITIN" | "CHITIN_UMBER" | "FLESH"))
        .map(String::as_str)
        .unwrap_or(if seed.is_multiple_of(2) {
            "V2_SHELL_RED"
        } else {
            "V2_SHELL_SCARLET"
        });
    let lobster = spec.affinity == "LOBSTER";
    // Keep native gait and limb IDs while replacing the spider-like silhouette.
    remove_where(body, |n| {
        n.id == "abdomen"
            || n.id == "petiole"
            || n.id.starts_with("fang_")
            || n.id.starts_with("shoulder_")
    });
    if let Some(t) = index(body, "torso") {
        let n = &mut body.nodes[t];
        n.rx = if lobster { 9.0 * s } else { 6.0 * s };
        n.ry = 3.2 * s;
        n.rz = Some(if lobster { 4.0 * s } else { 8.0 * s });
        n.material = shell.into();
    }
    for n in &mut body.nodes {
        if n.material.starts_with("CHITIN") {
            n.material = shell.into();
        }
        if matches!(n.kind.as_str(), "HIP" | "LIMB" | "FOOT") {
            n.material = shell.into();
            n.rx *= 1.2;
        }
        if n.kind == "HEAD" {
            n.rx = 2.4 * s;
            n.ry = 1.7 * s;
            n.rz = Some(2.6 * s);
        }
    }
    for i in 0..spec.limb_count.min(12) {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let rank = (i / 2) as f32;
        for (part, x, y, z) in [
            ("hip", (3.0 - rank * 2.1) * s, 0.0, side * 4.5 * s),
            ("limb", (4.0 - rank * 2.4) * s, 2.4 * s, side * 8.0 * s),
            ("foot", (5.5 - rank * 3.0) * s, 6.0 * s, side * 11.0 * s),
        ] {
            if let Some(k) = index(body, &format!("{part}_{i}")) {
                move_branch(body, k, x, y, z);
                body.nodes[k].rx = 0.8 * s;
                body.nodes[k].ry = 1.0 * s;
                body.nodes[k].rz = Some(0.7 * s);
            }
        }
    }
    for h in 0..spec.heads.min(5) {
        if let Some(i) = index(body, &format!("head_{h}")) {
            move_branch(body, i, 7.0 * s, -s, 0.0);
        }
    }
    if let Some(t) = index(body, "torso") {
        if lobster {
            let mut p = t;
            for k in 0..5 {
                p = landmark(
                    body,
                    p,
                    &format!("v2_lobster_tail_{k}"),
                    "TAIL",
                    [
                        (-7.0 - k as f32 * 2.7) * s,
                        (0.6 + k as f32 * 0.35) * s,
                        0.0,
                    ],
                    [
                        (3.2 - k as f32 * 0.28) * s,
                        2.3 * s,
                        (3.5 - k as f32 * 0.32) * s,
                    ],
                    shell,
                );
            }
            landmark(
                body,
                p,
                "v2_lobster_tail_fan",
                "FLUKE",
                [-21.5 * s, 2.0 * s, 0.0],
                [3.0 * s, 0.75 * s, 5.1 * s],
                shell,
            );
        }
        let claws = if has(spec, "EXPLICIT_ARMS") {
            spec.arm_count.min(8)
        } else {
            2
        };
        for i in 0..claws {
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let root = landmark(
                body,
                t,
                &format!("shoulder_{i}"),
                "SHOULDER",
                [5.0 * s, 0.0, side * 5.0 * s],
                [1.5 * s, 1.5 * s, 1.5 * s],
                shell,
            );
            let a = landmark(
                body,
                root,
                &format!("arm_{i}"),
                "ARM",
                [10.0 * s, 0.0, side * 7.0 * s],
                [1.6 * s, 1.7 * s, 1.7 * s],
                shell,
            );
            landmark(
                body,
                a,
                &format!("hand_{i}"),
                "HAND",
                [14.0 * s, -s, side * 7.7 * s],
                [4.0 * s, 2.4 * s, 2.0 * s],
                shell,
            );
        }
    }
}

fn anomaly(body: &mut Body, _seed: u64) {
    // One stable anatomical mistake per specimen; avoid random per-vertex damage.
    let side = "0";
    for n in &mut body.nodes {
        if n.id == format!("hand_{side}") {
            n.rx *= 1.55;
            n.ry *= 1.2;
            n.rz = Some(n.rz.unwrap_or(n.rx * 0.6) * 1.2);
        }
    }
}

fn chibi(body: &mut Body) {
    // Uniformly compact the rest rig first; enlarge whole head subtrees so ears,
    // horns, tusks and eyes remain seated on the same recognizable species head.
    for n in &mut body.nodes {
        n.x *= 0.42;
        n.y *= 0.56;
        n.rx *= 0.42;
        n.ry *= 0.56;
        n.z = n.z.map(|v| v * 0.56);
        n.rz = n.rz.map(|v| v * 0.56);
    }
    let original = body.nodes.clone();
    for i in 0..body.nodes.len() {
        let mut ancestor = Some(i);
        while let Some(a) = ancestor {
            if original[a].kind == "HEAD" {
                let head = &original[a];
                let n = &mut body.nodes[i];
                n.x = head.x + (n.x - head.x) * 2.7;
                n.y = head.y + (n.y - head.y) * 2.7;
                n.z = Some(
                    head.z.unwrap_or(0.0) + (n.z.unwrap_or(0.0) - head.z.unwrap_or(0.0)) * 2.7,
                );
                n.rx *= 2.7;
                if n.kind == "HEAD" {
                    n.rx *= 1.2;
                    n.ry *= 1.6;
                    n.rz = n.rz.map(|v| v * 1.3);
                }
                if n.id == "trunk" {
                    n.ry *= 0.55;
                    n.y = head.y + (n.y - head.y) * 0.6;
                }
                n.ry *= 2.7;
                n.rz = n.rz.map(|v| v * 2.7);
                break;
            }
            ancestor = original[a].parent;
        }
    }
}

fn fantasy(spec: &MonsterSpec, body: &mut Body, seed: u64, s: f32) {
    if spec.affinity == "CACODAEMON" {
        remove_where(body, |n| n.kind == "TENTACLE");
    }
    if matches!(
        spec.affinity.as_str(),
        "POTATO" | "ROUNDLING" | "CACODAEMON"
    ) {
        for n in &mut body.nodes {
            match n.kind.as_str() {
                "TORSO" => {
                    n.rx = 5.0 * s;
                    n.ry = 5.0 * s;
                    n.rz = Some(4.0 * s);
                }
                "NECK" => {
                    n.rx = 1.0 * s;
                    n.ry = 1.0 * s;
                    n.rz = Some(1.0 * s);
                }
                "HEAD" => {
                    n.rx = if spec.affinity == "CACODAEMON" {
                        9.5 * s
                    } else {
                        7.5 * s
                    };
                    n.ry = if spec.affinity == "POTATO" {
                        9.5 * s
                    } else {
                        7.5 * s
                    };
                    n.rz = Some(7.0 * s);
                }
                "ARM" | "SHOULDER" => {
                    n.rx *= 0.75;
                    n.ry *= 0.85;
                }
                "LIMB" | "SHIN" => {
                    n.ry *= 0.55;
                    n.y *= 0.7;
                }
                "FOOT" => {
                    n.y *= 0.8;
                    n.rx *= 1.15;
                }
                _ => {}
            }
        }
        if let Some(h) = index(body, "head_0") {
            move_branch(body, h, 2.0 * s, -2.0 * s, 0.0);
        }
        if spec.affinity == "CACODAEMON" {
            if let Some(t) = index(body, "torso") {
                body.nodes[t].rx = 3.0 * s;
                body.nodes[t].ry = 3.0 * s;
            }
        }
    }
    if spec.affinity == "ROBOT" {
        for n in &mut body.nodes {
            if matches!(n.kind.as_str(), "ARM" | "LIMB" | "SHIN" | "SHOULDER") {
                let k =
                    n.id.bytes()
                        .fold(seed, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
                n.rx *= 0.8 + (k % 5) as f32 * 0.16;
                n.ry *= 0.9 + ((k >> 8) % 3) as f32 * 0.12;
                n.angle += (k % 7) as f32 * 0.06 - 0.18;
            }
        }
    }
    if spec.affinity == "GORGON" {
        for h in 0..spec.heads.min(5) {
            if let Some(head) = index(body, &format!("head_{h}")) {
                let n = body.nodes[head].clone();
                for i in 0..5 {
                    let z = (i as f32 - 2.0) * 1.45 * s;
                    let root = landmark(
                        body,
                        head,
                        &format!("v2_gorgon_snake_{h}_{i}"),
                        "TENTACLE",
                        [n.x - 1.0 * s, n.y - 4.0 * s, z],
                        [0.6 * s, 2.5 * s, 0.6 * s],
                        "SCALES",
                    );
                    landmark(
                        body,
                        root,
                        &format!("v2_gorgon_snake_tip_{h}_{i}"),
                        "LOBE",
                        [
                            n.x + (i % 2) as f32 * 2.0 * s,
                            n.y - (6.0 + (i % 2) as f32) * s,
                            z,
                        ],
                        [1.2 * s, 0.65 * s, 0.7 * s],
                        "SCALES",
                    );
                }
            }
        }
    }
    if matches!(spec.affinity.as_str(), "DEMONIC" | "CACODAEMON") {
        remove_where(body, |n| n.id == "horn_0");
        if let Some(h) = index(body, "head_0") {
            let n = body.nodes[h].clone();
            for (i, z) in [-1.0, 1.0].into_iter().enumerate() {
                landmark(
                    body,
                    h,
                    &format!("v2_demon_horn_{i}"),
                    "HORN",
                    [
                        n.x - n.rx * 0.3,
                        n.y - n.ry * 0.8,
                        z * n.rz.unwrap_or(n.ry * 0.88) * 0.7,
                    ],
                    [1.1 * s, 3.4 * s, 1.1 * s],
                    "HORN",
                );
            }
        }
    }
    if spec.affinity == "REAPER" {
        if index(body, "weapon").is_none() {
            if let Some(h) = index(body, "hand_0") {
                let n = body.nodes[h].clone();
                let w = landmark(
                    body,
                    h,
                    "weapon",
                    "WEAPON",
                    [
                        n.x + (0.52 * 9.0 * s) * (-1.2_f32).cos(),
                        n.y + (0.52 * 9.0 * s) * (-1.2_f32).sin(),
                        n.z.unwrap_or(0.0),
                    ],
                    [9.0 * s, 3.0 * s, 1.0 * s],
                    "METAL",
                );
                body.nodes[w].angle = -1.2;
            }
        }
        for n in &mut body.nodes {
            if matches!(
                n.kind.as_str(),
                "TORSO" | "ARM" | "LIMB" | "SHIN" | "SHOULDER" | "HIP"
            ) {
                n.material = "LEATHER_BLACK".into();
            }
        }
        if let Some(t) = index(body, "torso") {
            landmark(
                body,
                t,
                "v2_reaper_robe",
                "SKIRT",
                [0.0, 5.0 * s, 0.0],
                [7.0 * s, 8.0 * s, 4.2 * s],
                "LEATHER_BLACK",
            );
        }
    }
    if has(spec, "DANCER") {
        for h in 0..spec.heads.min(5) {
            if let Some(head) = index(body, &format!("head_{h}")) {
                let n = body.nodes[head].clone();
                landmark(
                    body,
                    head,
                    &format!("v2_dancer_sunglasses_{h}"),
                    "SUNGLASSES",
                    [n.x + n.rx * 0.86, n.y - n.ry * 0.15, n.z.unwrap_or(0.0)],
                    [n.rx * 0.35, n.ry * 0.38, n.rz.unwrap_or(n.ry * 0.88)],
                    "LEATHER_BLACK",
                );
            }
        }
        if let Some(t) = index(body, "torso") {
            let n = body.nodes[t].clone();
            landmark(
                body,
                t,
                "v2_dancer_dollar_chain",
                "BLING",
                [n.x + n.rx * 1.0, n.y - n.ry * 0.15, n.z.unwrap_or(0.0)],
                [0.7 * s, 3.0 * s, 3.0 * s],
                "METAL",
            );
        }
    }
    if has(spec, "NINJA") {
        for h in 0..spec.heads.min(5) {
            if let Some(head) = index(body, &format!("head_{h}")) {
                let n = body.nodes[head].clone();
                landmark(
                    body,
                    head,
                    &format!("v2_ninja_mask_{h}"),
                    "MASK",
                    [n.x + n.rx * 0.8, n.y + n.ry * 0.25, n.z.unwrap_or(0.0)],
                    [n.rx * 0.3, n.ry * 0.4, n.rz.unwrap_or(n.ry * 0.85) * 0.9],
                    "LEATHER_BLACK",
                );
            }
        }
    }
    if has(spec, "FLAMING") {
        let target = index(body, "weapon")
            .or_else(|| index(body, "hand_0"))
            .or_else(|| index(body, "head_0"));
        if let Some(p) = target {
            let n = body.nodes[p].clone();
            for i in 0..3 {
                landmark(
                    body,
                    p,
                    &format!("v2_flame_{i}"),
                    "FLAME",
                    [
                        n.x + (i as f32 - 1.0) * n.rx * 0.5,
                        n.y - n.ry * 0.6,
                        n.z.unwrap_or(0.0),
                    ],
                    [0.9 * s, (1.7 + (i % 2) as f32) * s, 0.7 * s],
                    "FIRE",
                );
            }
        }
    }
}

fn spatial_humanoid(spec: &MonsterSpec, body: &mut Body, s: f32) {
    // Legacy arms spread along the forward axis and collapse in the front view.
    // Seat paired chains on the lateral axis before fixed-length rigging.
    if let Some(t) = index(body, "torso") {
        let n = &mut body.nodes[t];
        n.rx *= 0.65;
        n.rz = Some(if spec.affinity == "ANIME_GIRL" {
            4.4 * s
        } else {
            5.2 * s
        });
    }
    for i in 0..spec.arm_count.min(8) {
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        let rank = (i / 2) as f32;
        let width = if spec.affinity == "TROLL" {
            2.0
        } else if matches!(spec.affinity.as_str(), "ORC" | "MINOTAUR") {
            1.0
        } else {
            0.0
        };
        for (part, x, y, z) in [
            (
                "shoulder",
                0.0,
                -5.2 + rank * 3.0,
                side * (5.5 + width + rank * 0.3),
            ),
            (
                "arm",
                1.0,
                -1.3 + rank * 3.0,
                side * (6.5 + width + rank * 0.4),
            ),
            (
                "hand",
                2.3,
                2.5 + rank * 3.0,
                side * (6.8 + width + rank * 0.5),
            ),
        ] {
            if let Some(k) = index(body, &format!("{part}_{i}")) {
                move_branch(body, k, x * s, y * s, z * s);
            }
        }
    }
    for i in 0..spec.limb_count.min(12) {
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        let rank = (i / 2) as f32;
        for (part, x, y) in [
            ("leg_root", 0.0, 4.0),
            ("limb", 0.5, 7.5),
            ("shin", 0.2, 11.0),
            ("foot", 1.4, 14.0),
        ] {
            if let Some(k) = index(body, &format!("{part}_{i}")) {
                move_branch(
                    body,
                    k,
                    (x - rank * 1.5) * s,
                    y * s,
                    side * (2.8 + rank * 1.5) * s,
                );
            }
        }
    }
}

pub(super) fn mounted_modifiers(spec: &MonsterSpec, body: &mut Body, s: f32) {
    if !has(spec, "DANCER") {
        return;
    }
    if let Some(h) = index(body, "rider_head_0").or_else(|| index(body, "head_0")) {
        let n = body.nodes[h].clone();
        landmark(
            body,
            h,
            "v2_dancer_sunglasses_mount",
            "SUNGLASSES",
            [n.x + n.rx * 0.86, n.y - n.ry * 0.15, n.z.unwrap_or(0.0)],
            [n.rx * 0.35, n.ry * 0.38, n.rz.unwrap_or(n.ry * 0.88)],
            "LEATHER_BLACK",
        );
    }
    if let Some(t) = index(body, "rider_torso").or_else(|| index(body, "torso")) {
        let n = body.nodes[t].clone();
        landmark(
            body,
            t,
            "v2_dancer_dollar_chain_mount",
            "BLING",
            [n.x + n.rx, n.y, n.z.unwrap_or(0.0)],
            [0.7 * s, 3.0 * s, 3.0 * s],
            "METAL",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    #[test]
    fn crustacean_red_default_and_material_override() {
        for prompt in ["crab", "lobster"] {
            let spec = crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(0));
            crustacean(&spec, &mut body, 1.0, 0);
            assert_eq!(
                body.nodes[index(&body, "torso").unwrap()].material,
                "V2_SHELL_RED"
            );
            let mut explicit = spec.clone();
            explicit.materials = vec!["METAL".into()];
            crustacean(&explicit, &mut body, 1.0, 0);
            assert_eq!(body.nodes[index(&body, "torso").unwrap()].material, "METAL");
        }
    }
    #[test]
    fn chibi_compacts_body_and_enlarges_whole_species_head() {
        for prompt in ["elephant", "wolf", "orc", "anime girl"] {
            let spec = crate::parser::parse_vocabulary(prompt);
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(0));
            let before = body.clone();
            chibi(&mut body);
            for (a, b) in before.nodes.iter().zip(&body.nodes) {
                if a.kind == "HEAD" {
                    assert!((b.rx / a.rx - 1.3608).abs() < 0.001, "{prompt}");
                }
                if a.kind == "TORSO" {
                    assert!((b.rx / a.rx - 0.42).abs() < 0.001, "{prompt}");
                }
            }
        }
    }
}
