//! A deliberately small, deterministic Wavefront importer for opaque prop meshes.
//!
//! Supported geometry is polygonal OBJ (positions, optional UVs/normals, positive
//! and relative indices). Simple planar polygons are triangulated by ear clipping.
//! Zero-area exporter triangles are discarded; wholly degenerate meshes are rejected.
//! MTL diffuse `Kd` colors are supported; lighting parameters are ignored because
//! the prop baker owns lighting. Texture maps, vertex colors, transparency and
//! free-form geometry require conversion to opaque, flat-colored polygons first.
//! Inputs are bounded to 64 MiB OBJ, 4 MiB combined MTL, 256 material files,
//! 1M positions/UVs/normals/polygons/output triangles, and 256 materials including
//! the neutral fallback. Individual polygons are limited to 4096 vertices and
//! positions must be finite and within +/-1,000,000 in source units.

use super::{PropError, PropFace, PropMaterial, PropMesh};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path};

const MAX_OBJ_BYTES: usize = 64 * 1024 * 1024;
const MAX_MATERIAL_BYTES: usize = 4 * 1024 * 1024;
const MAX_LOGICAL_LINE_BYTES: usize = 1024 * 1024;
const MAX_VERTICES: usize = 1_000_000;
const MAX_FACES: usize = 1_000_000;
const MAX_MATERIALS: usize = 256;
const MAX_MATERIAL_FILES: usize = 256;
const MAX_POLYGON_VERTICES: usize = 4096;
const EPSILON: f64 = 1e-12;

/// Load an OBJ and the MTL files it names. Material files must be inside the OBJ's
/// directory (subdirectories are allowed); external paths and symlink escapes are
/// rejected. Copy external materials alongside the OBJ before importing them.
pub fn load_obj(path: &Path) -> Result<PropMesh, PropError> {
    let source = read_limited(path, MAX_OBJ_BYTES, "OBJ")?;
    let base = fs::canonicalize(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    let mut materials = String::new();
    let mut loaded = HashSet::new();
    let mut material_references = 0;
    for logical_line in logical_lines(&source) {
        let (line_number, line) = logical_line?;
        let (command, rest) = statement(&line);
        if command != "mtllib" {
            continue;
        }
        if rest.is_empty() {
            return Err(invalid(
                "OBJ",
                line_number,
                "mtllib needs a material filename",
            ));
        }
        // Blender and other exporters sometimes leave a single filename with
        // spaces unquoted. Prefer that exact local filename when it exists.
        let normalized = rest.replace('\\', "/");
        let names = if !normalized.contains('"')
            && safe_relative(Path::new(&normalized))
            && base.join(&normalized).is_file()
        {
            vec![normalized]
        } else {
            material_filenames(rest, line_number)?
        };
        for name in names {
            check_count(
                material_references,
                MAX_MATERIAL_FILES,
                "material file references",
                "OBJ",
                line_number,
            )?;
            material_references += 1;
            let normalized = name.replace('\\', "/");
            let relative = Path::new(&normalized);
            if !safe_relative(relative) {
                return Err(invalid("OBJ", line_number, &format!("material path '{name}' must stay inside the OBJ directory; copy the MTL beside the OBJ and update mtllib")));
            }
            let candidate = fs::canonicalize(base.join(relative)).map_err(|error| {
                invalid("OBJ", line_number, &format!("cannot open material file '{name}': {error}; supply the referenced MTL alongside the OBJ"))
            })?;
            if !candidate.starts_with(&base) {
                return Err(invalid(
                    "OBJ",
                    line_number,
                    "material symlink leaves the OBJ directory; copy the MTL alongside the OBJ",
                ));
            }
            if !loaded.contains(&candidate) {
                check_count(
                    loaded.len(),
                    MAX_MATERIAL_FILES,
                    "material files",
                    "OBJ",
                    line_number,
                )?;
                loaded.insert(candidate.clone());
                if !materials.is_empty() {
                    check_bytes(materials.len() + 1, MAX_MATERIAL_BYTES, "combined MTL")?;
                    materials.push('\n');
                }
                let remaining = MAX_MATERIAL_BYTES - materials.len();
                let contents = read_limited(&candidate, remaining, "combined MTL")?;
                materials.push_str(&contents);
            }
        }
    }
    parse_obj(&source, &materials)
}

/// Parse an OBJ with the complete contents of its material libraries supplied by
/// the caller. `mtllib` declarations are not loaded by this in-memory entry point.
/// Faces without `usemtl` use an opaque neutral material. Normals and UVs are
/// validated but the baker recomputes flat normals and uses material colors.
pub fn parse_obj(source: &str, materials: &str) -> Result<PropMesh, PropError> {
    check_bytes(source.len(), MAX_OBJ_BYTES, "OBJ")?;
    check_bytes(materials.len(), MAX_MATERIAL_BYTES, "combined MTL")?;
    let (materials, material_names) = parse_mtl(materials)?;
    let mut mesh = PropMesh {
        vertices: Vec::new(),
        faces: Vec::new(),
        materials,
    };
    let mut current_material = 0;
    let mut texture_count = 0;
    let mut normal_count = 0;
    let mut polygon_count = 0;
    for logical_line in logical_lines(source) {
        let (line_number, line) = logical_line?;
        let (command, rest) = statement(&line);
        match command {
            "" | "#" | "o" | "g" | "s" | "mtllib" => {}
            "v" => {
                check_count(mesh.vertices.len(), MAX_VERTICES, "positions", "OBJ", line_number)?;
                let components = numbers(rest, "OBJ", line_number)?;
                if components.len() != 3 && components.len() != 4 {
                    return Err(invalid("OBJ", line_number, "v requires x y z and optional homogeneous w; vertex colors are unsupported, assign an opaque MTL Kd material instead"));
                }
                let w = components.get(3).copied().unwrap_or(1.0);
                if w == 0.0 {
                    return Err(invalid("OBJ", line_number, "vertex homogeneous w must not be zero"));
                }
                let vertex = [components[0] / w, components[1] / w, components[2] / w];
                if vertex.iter().any(|value| !value.is_finite()) {
                    return Err(invalid("OBJ", line_number, "vertex coordinates are outside the supported finite f32 range"));
                }
                if vertex.iter().any(|value| value.abs() > 1_000_000.0) {
                    return Err(invalid("OBJ", line_number, "positions must be within +/-1,000,000; rescale the asset before importing"));
                }
                mesh.vertices.push(vertex);
            }
            "vt" => {
                check_count(texture_count, MAX_VERTICES, "texture coordinates", "OBJ", line_number)?;
                let components = numbers(rest, "OBJ", line_number)?;
                if !(1..=3).contains(&components.len()) {
                    return Err(invalid("OBJ", line_number, "vt requires one to three finite texture coordinates"));
                }
                texture_count += 1;
            }
            "vn" => {
                check_count(normal_count, MAX_VERTICES, "normals", "OBJ", line_number)?;
                if numbers(rest, "OBJ", line_number)?.len() != 3 {
                    return Err(invalid("OBJ", line_number, "vn requires three finite normal coordinates"));
                }
                normal_count += 1;
            }
            "usemtl" => {
                current_material = *material_names.get(rest).ok_or_else(|| {
                    invalid("OBJ", line_number, &format!("unknown material '{rest}'; include its newmtl/Kd definition in the referenced MTL"))
                })?;
            }
            "f" => {
                check_count(polygon_count, MAX_FACES, "polygons", "OBJ", line_number)?;
                polygon_count += 1;
                let mut indices = Vec::new();
                for token in rest.split_whitespace() {
                    if indices.len() == MAX_POLYGON_VERTICES {
                        return Err(invalid("OBJ", line_number, "polygon exceeds 4096 vertices; triangulate it in your modeling tool before export"));
                    }
                    let fields: Vec<_> = token.split('/').take(4).collect();
                    if fields.len() > 3 || fields[0].is_empty() || (fields.len() == 2 && fields[1].is_empty()) || (fields.len() == 3 && fields[2].is_empty()) {
                        return Err(invalid("OBJ", line_number, &format!("invalid face reference '{token}'; expected v, v/vt, v//vn or v/vt/vn")));
                    }
                    indices.push(index(fields[0], mesh.vertices.len(), "position", line_number)?);
                    if fields.len() >= 2 && !fields[1].is_empty() {
                        index(fields[1], texture_count, "texture coordinate", line_number)?;
                    }
                    if fields.len() == 3 {
                        index(fields[2], normal_count, "normal", line_number)?;
                    }
                }
                for triangle in triangulate(&indices, &mesh.vertices, line_number)? {
                    check_count(mesh.faces.len(), MAX_FACES, "output triangles", "OBJ", line_number)?;
                    mesh.faces.push(PropFace { indices: triangle, material: current_material });
                }
            }
            "l" | "p" | "curv" | "curv2" | "surf" | "cstype" | "deg" | "bmat" | "step" | "parm" | "trim" | "hole" | "scrv" | "sp" | "end" | "con" => {
                return Err(invalid("OBJ", line_number, &format!("unsupported geometry statement '{command}'; convert curves, points and lines to a polygon mesh before export")));
            }
            // Common exporter metadata does not change mesh geometry/materials.
            "bevel" | "c_interp" | "d_interp" | "lod" | "mg" => {}
            _ => return Err(invalid("OBJ", line_number, &format!("unsupported statement '{command}'; export a polygonal Wavefront OBJ with opaque Kd materials"))),
        }
    }
    if mesh.vertices.is_empty() || mesh.faces.is_empty() {
        return Err(PropError::Invalid(
            "OBJ must contain vertices and at least one non-degenerate polygon face".into(),
        ));
    }
    super::validate_mesh(&mesh)?;
    Ok(mesh)
}

fn invalid(format: &str, line: usize, message: &str) -> PropError {
    PropError::Invalid(format!("{format} line {line}: {message}"))
}

fn statement(line: &str) -> (&str, &str) {
    let line = line.trim();
    let split = line.find(char::is_whitespace).unwrap_or(line.len());
    (&line[..split], line[split..].trim())
}

fn check_bytes(size: usize, maximum: usize, kind: &str) -> Result<(), PropError> {
    if size > maximum {
        return Err(PropError::Invalid(format!(
            "{kind} exceeds the {maximum}-byte input budget; split or simplify the asset"
        )));
    }
    Ok(())
}

fn check_count(
    count: usize,
    maximum: usize,
    kind: &str,
    format: &str,
    line: usize,
) -> Result<(), PropError> {
    if count >= maximum {
        return Err(invalid(
            format,
            line,
            &format!("too many {kind}; limit is {maximum}; split or simplify the asset"),
        ));
    }
    Ok(())
}

fn read_limited(path: &Path, maximum: usize, kind: &str) -> Result<String, PropError> {
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(PropError::Invalid(format!(
            "{kind} input must be a regular file"
        )));
    }
    // Check metadata first, but also bound the actual read if a file changes or
    // the filesystem reports an inaccurate length. Never trust metadata alone.
    if metadata.len() > maximum as u64 {
        return Err(PropError::Invalid(format!(
            "{kind} exceeds the {maximum}-byte input budget; split or simplify the asset"
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(maximum as u64 + 1).read_to_end(&mut bytes)?;
    check_bytes(bytes.len(), maximum, kind)?;
    String::from_utf8(bytes)
        .map_err(|_| PropError::Invalid(format!("{kind} must be valid UTF-8 text")))
}

fn logical_lines(source: &str) -> impl Iterator<Item = Result<(usize, String), PropError>> + '_ {
    let mut physical_lines = source.trim_start_matches('\u{feff}').lines().enumerate();
    std::iter::from_fn(move || {
        let mut pending = String::new();
        let mut start = 0;
        for (offset, raw) in physical_lines.by_ref() {
            let line = raw.split('#').next().unwrap_or("").trim_end();
            if pending.is_empty() {
                start = offset + 1;
            }
            if pending.len().saturating_add(line.len()) > MAX_LOGICAL_LINE_BYTES {
                return Some(Err(PropError::Invalid(format!(
                    "line {start}: logical line exceeds 1 MiB; split or simplify the asset"
                ))));
            }
            if let Some(prefix) = line.strip_suffix('\\') {
                pending.push_str(prefix);
                pending.push(' ');
            } else {
                pending.push_str(line);
                return Some(Ok((start, pending)));
            }
        }
        if !pending.is_empty() {
            Some(Err(PropError::Invalid(format!(
                "line {start}: unfinished line continuation"
            ))))
        } else {
            None
        }
    })
}

fn numbers(rest: &str, format: &str, line: usize) -> Result<Vec<f32>, PropError> {
    let mut values = Vec::with_capacity(4);
    for token in rest.split_whitespace() {
        if values.len() == 4 {
            return Err(invalid(format, line, "too many numeric components; expected at most four (vertex colors are unsupported)"));
        }
        values.push(
            token
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    invalid(format, line, &format!("'{token}' is not a finite number"))
                })?,
        );
    }
    Ok(values)
}

fn index(token: &str, count: usize, kind: &str, line: usize) -> Result<usize, PropError> {
    let value = token
        .parse::<i64>()
        .map_err(|_| invalid("OBJ", line, &format!("invalid {kind} index '{token}'")))?;
    let resolved = if value > 0 {
        value - 1
    } else if value < 0 {
        count as i64 + value
    } else {
        -1
    };
    if resolved < 0 || resolved as usize >= count {
        return Err(invalid("OBJ", line, &format!("{kind} index {value} is out of range for {count} previously declared values (OBJ indices start at 1)")));
    }
    Ok(resolved as usize)
}

fn safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.to_string_lossy().contains(':')
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn material_filenames(rest: &str, line: usize) -> Result<Vec<String>, PropError> {
    let mut names = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in rest.chars() {
        if character == '"' {
            quoted = !quoted;
        } else if character.is_whitespace() && !quoted {
            if !current.is_empty() {
                check_count(
                    names.len(),
                    MAX_MATERIAL_FILES,
                    "material files",
                    "OBJ",
                    line,
                )?;
                names.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if quoted {
        return Err(invalid("OBJ", line, "unterminated quoted mtllib filename"));
    }
    if !current.is_empty() {
        check_count(
            names.len(),
            MAX_MATERIAL_FILES,
            "material files",
            "OBJ",
            line,
        )?;
        names.push(current);
    }
    if names.is_empty() {
        return Err(invalid(
            "OBJ",
            line,
            "mtllib needs a nonempty material filename",
        ));
    }
    Ok(names)
}

fn parse_mtl(source: &str) -> Result<(Vec<PropMaterial>, HashMap<String, usize>), PropError> {
    let mut materials = vec![PropMaterial {
        name: "__default".into(),
        color: [200, 200, 200, 255],
        surface: "NONE".into(),
    }];
    let mut names = HashMap::new();
    let mut current = None;
    for logical_line in logical_lines(source) {
        let (line_number, line) = logical_line?;
        let (command, rest) = statement(&line);
        if command.is_empty() {
            continue;
        }
        if command == "newmtl" {
            check_count(
                materials.len(),
                MAX_MATERIALS,
                "materials (including neutral fallback)",
                "MTL",
                line_number,
            )?;
            if rest.is_empty() || names.contains_key(rest) {
                return Err(invalid(
                    "MTL",
                    line_number,
                    "newmtl requires a nonempty, unique material name",
                ));
            }
            let material_index = materials.len();
            names.insert(rest.to_owned(), material_index);
            materials.push(PropMaterial {
                name: rest.into(),
                color: [200, 200, 200, 255],
                surface: "NONE".into(),
            });
            current = Some(material_index);
            continue;
        }
        let material_index = current.ok_or_else(|| {
            invalid(
                "MTL",
                line_number,
                "material properties must follow a newmtl statement",
            )
        })?;
        if command.to_ascii_lowercase().starts_with("map_")
            || matches!(
                command.to_ascii_lowercase().as_str(),
                "bump" | "disp" | "decal" | "refl" | "norm"
            )
        {
            return Err(invalid("MTL", line_number, &format!("texture statement '{command}' is unsupported; bake/convert the asset to opaque per-face Kd colors and remove texture maps before importing")));
        }
        match command {
            "Kd" => {
                let values = numbers(rest, "MTL", line_number)?;
                if values.len() != 3 || values.iter().any(|value| !(0.0..=1.0).contains(value)) {
                    return Err(invalid(
                        "MTL",
                        line_number,
                        "Kd requires three RGB values between 0 and 1",
                    ));
                }
                for (channel, value) in values.iter().enumerate() {
                    materials[material_index].color[channel] = (value * 255.0).round() as u8;
                }
            }
            "d" | "Tr" => {
                let values = numbers(
                    rest.strip_prefix("-halo ").unwrap_or(rest),
                    "MTL",
                    line_number,
                )?;
                let opaque = if command == "d" { 1.0 } else { 0.0 };
                if values.len() != 1 || values[0] != opaque {
                    return Err(invalid("MTL", line_number, "transparent materials are unsupported; make the material opaque (d 1 or Tr 0) before importing"));
                }
            }
            // Material lighting and PBR scalar settings are intentionally replaced
            // by the deterministic baker's lighting; validate their numeric input.
            "Ka" | "Ks" | "Ke" | "Tf" => {
                let values = numbers(rest, "MTL", line_number)?;
                if values.len() != 3 {
                    return Err(invalid(
                        "MTL",
                        line_number,
                        &format!("{command} requires three finite RGB values"),
                    ));
                }
            }
            "Ns" | "Ni" | "Pr" | "Pm" | "Ps" | "Pc" | "Pcr" | "aniso" | "anisor" | "illum" => {
                let values = numbers(rest, "MTL", line_number)?;
                if values.len() != 1 {
                    return Err(invalid(
                        "MTL",
                        line_number,
                        &format!("{command} requires one finite number"),
                    ));
                }
                if command == "illum" && matches!(values[0], 4.0 | 6.0 | 7.0 | 9.0) {
                    return Err(invalid("MTL", line_number, "transparent/refractive illumination models are unsupported; export opaque materials with illum 0, 1 or 2"));
                }
            }
            _ => {
                return Err(invalid(
                    "MTL",
                    line_number,
                    &format!(
                    "unsupported material statement '{command}'; export opaque RGB Kd materials"
                ),
                ))
            }
        }
    }
    Ok((materials, names))
}

fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    cross(a, b, p).abs() <= EPSILON
        && (0..2).all(|axis| {
            p[axis] >= a[axis].min(b[axis]) - EPSILON && p[axis] <= a[axis].max(b[axis]) + EPSILON
        })
}

fn intersects(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let ab_c = cross(a, b, c);
    let ab_d = cross(a, b, d);
    let cd_a = cross(c, d, a);
    let cd_b = cross(c, d, b);
    (ab_c * ab_d < -EPSILON * EPSILON && cd_a * cd_b < -EPSILON * EPSILON)
        || on_segment(a, b, c)
        || on_segment(a, b, d)
        || on_segment(c, d, a)
        || on_segment(c, d, b)
}

fn triangulate(
    indices: &[usize],
    vertices: &[[f32; 3]],
    line: usize,
) -> Result<Vec<[usize; 3]>, PropError> {
    let bad_polygon = |detail: &str| {
        invalid(
            "OBJ",
            line,
            &format!(
                "{detail}; triangulate/repair the polygon in your modeling tool before export"
            ),
        )
    };
    if indices.len() < 3 {
        return Err(bad_polygon("face requires at least three vertices"));
    }
    let mut unique = HashSet::new();
    if indices.iter().any(|index| !unique.insert(*index)) {
        return Err(bad_polygon("face repeats a position index"));
    }
    // Normalize in f64 before geometric predicates. This avoids overflow with
    // finite f32 inputs and keeps the tolerances independent of authoring units.
    let origin = vertices[indices[0]].map(f64::from);
    let mut positions: Vec<[f64; 3]> = indices
        .iter()
        .map(|&index| {
            let p = vertices[index];
            [
                f64::from(p[0]) - origin[0],
                f64::from(p[1]) - origin[1],
                f64::from(p[2]) - origin[2],
            ]
        })
        .collect();
    let scale = positions
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if scale == 0.0 {
        return if indices.len() == 3 {
            Ok(Vec::new())
        } else {
            Err(bad_polygon("face has zero area"))
        };
    }
    for position in &mut positions {
        for coordinate in position {
            *coordinate /= scale;
        }
    }
    let count = positions.len();
    let mut normal = [0.0_f64; 3];
    for i in 0..count {
        let a = positions[i];
        let b = positions[(i + 1) % count];
        normal[0] += (a[1] - b[1]) * (a[2] + b[2]);
        normal[1] += (a[2] - b[2]) * (a[0] + b[0]);
        normal[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    let length = normal.iter().map(|value| value * value).sum::<f64>().sqrt();
    if count == 3 && length == 0.0 {
        // Some polygon exporters emit collinear triangles along bevels. These
        // contribute no surface and are safe to omit without repairing topology.
        return Ok(Vec::new());
    }
    if length <= EPSILON {
        return Err(bad_polygon("face is degenerate or self-intersecting"));
    }
    for value in &mut normal {
        *value /= length;
    }
    if positions
        .iter()
        .any(|p| (p[0] * normal[0] + p[1] * normal[1] + p[2] * normal[2]).abs() > 1e-4)
    {
        return Err(bad_polygon("face is non-planar"));
    }
    let dominant = (0..3)
        .max_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))
        .unwrap();
    let points: Vec<[f64; 2]> = positions
        .iter()
        .map(|p| match dominant {
            0 => [p[1], p[2]],
            1 => [p[0], p[2]],
            _ => [p[0], p[1]],
        })
        .collect();
    for i in 0..count {
        let next_i = (i + 1) % count;
        if (points[i][0] - points[next_i][0])
            .abs()
            .max((points[i][1] - points[next_i][1]).abs())
            <= EPSILON
        {
            return Err(bad_polygon("face contains a zero-length edge"));
        }
        for j in (i + 1)..count {
            let next_j = (j + 1) % count;
            if next_i != j
                && next_j != i
                && intersects(points[i], points[next_i], points[j], points[next_j])
            {
                return Err(bad_polygon("face is self-intersecting"));
            }
        }
    }
    let area: f64 = (0..count)
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % count];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum();
    if area.abs() <= EPSILON {
        return Err(bad_polygon("face has zero projected area"));
    }
    let winding = area.signum();
    let mut remaining: Vec<usize> = (0..count).collect();
    let mut triangles = Vec::with_capacity(count - 2);
    while remaining.len() > 3 {
        let mut removed = false;
        for i in 0..remaining.len() {
            let a = remaining[(i + remaining.len() - 1) % remaining.len()];
            let b = remaining[i];
            let c = remaining[(i + 1) % remaining.len()];
            let turn = cross(points[a], points[b], points[c]) * winding;
            if turn.abs() <= EPSILON && on_segment(points[a], points[c], points[b]) {
                remaining.remove(i);
                removed = true;
                break;
            }
            if turn <= EPSILON {
                continue;
            }
            let contains_vertex = remaining.iter().any(|&p| {
                p != a
                    && p != b
                    && p != c
                    && cross(points[a], points[b], points[p]) * winding >= -EPSILON
                    && cross(points[b], points[c], points[p]) * winding >= -EPSILON
                    && cross(points[c], points[a], points[p]) * winding >= -EPSILON
            });
            if !contains_vertex {
                triangles.push([indices[a], indices[b], indices[c]]);
                remaining.remove(i);
                removed = true;
                break;
            }
        }
        if !removed {
            return Err(bad_polygon(
                "cannot triangulate an invalid or numerically degenerate face",
            ));
        }
    }
    if cross(
        points[remaining[0]],
        points[remaining[1]],
        points[remaining[2]],
    ) * winding
        > EPSILON
    {
        triangles.push([
            indices[remaining[0]],
            indices[remaining[1]],
            indices[remaining[2]],
        ]);
    }
    if triangles.is_empty() {
        return Err(bad_polygon("face has zero area"));
    }
    Ok(triangles)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRIANGLE: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

    fn error(source: &str, materials: &str) -> String {
        match parse_obj(source, materials) {
            Err(PropError::Invalid(message)) => message,
            Err(other) => panic!("unexpected error: {other:?}"),
            Ok(_) => panic!("expected invalid input"),
        }
    }

    #[test]
    fn public_parsers_reject_oversized_inputs_before_parsing() {
        assert!(error(&" ".repeat(MAX_OBJ_BYTES + 1), "").contains("input budget"));
        assert!(error(TRIANGLE, &" ".repeat(MAX_MATERIAL_BYTES + 1)).contains("input budget"));
        assert!(
            error(&format!("o {}", "x".repeat(MAX_LOGICAL_LINE_BYTES)), "")
                .contains("logical line exceeds")
        );
    }

    #[test]
    fn capped_reads_reject_oversized_files_and_combined_materials() {
        let directory = tempfile::tempdir().unwrap();
        let obj = directory.path().join("prop.obj");
        fs::File::create(&obj)
            .unwrap()
            .set_len(MAX_OBJ_BYTES as u64 + 1)
            .unwrap();
        assert!(
            matches!(load_obj(&obj), Err(PropError::Invalid(message)) if message.contains("input budget"))
        );
        fs::write(&obj, format!("mtllib first.mtl second.mtl\n{TRIANGLE}")).unwrap();
        fs::write(
            directory.path().join("first.mtl"),
            vec![b'\n'; MAX_MATERIAL_BYTES / 2 + 1],
        )
        .unwrap();
        fs::write(
            directory.path().join("second.mtl"),
            vec![b'\n'; MAX_MATERIAL_BYTES / 2],
        )
        .unwrap();
        assert!(
            matches!(load_obj(&obj), Err(PropError::Invalid(message)) if message.contains("combined MTL") && message.contains("input budget"))
        );
    }

    #[test]
    fn enforces_material_and_filename_limits_incrementally() {
        let materials: String = (0..MAX_MATERIALS)
            .map(|i| format!("newmtl material{i}\n"))
            .collect();
        assert!(error(TRIANGLE, &materials).contains("too many materials"));
        let names = "material.mtl ".repeat(MAX_MATERIAL_FILES + 1);
        assert!(
            matches!(material_filenames(&names, 1), Err(PropError::Invalid(message)) if message.contains("too many material files"))
        );
        for (maximum, kind) in [(MAX_VERTICES, "positions"), (MAX_FACES, "output triangles")] {
            assert!(check_count(maximum - 1, maximum, kind, "OBJ", 1).is_ok());
            assert!(
                matches!(check_count(maximum, maximum, kind, "OBJ", 1), Err(PropError::Invalid(message)) if message.contains(kind))
            );
        }
    }

    #[test]
    fn stops_at_position_limit_in_public_parser() {
        let source = "v 0 0 0\n".repeat(MAX_VERTICES + 1);
        assert!(error(&source, "").contains("too many positions"));
    }

    #[test]
    fn stops_at_polygon_vertex_limit_before_triangulation() {
        let source = format!("v 0 0 0\nf {}", "1 ".repeat(MAX_POLYGON_VERTICES + 1));
        assert!(error(&source, "").contains("polygon exceeds 4096 vertices"));
    }

    #[test]
    fn imports_opaque_diffuse_material_and_relative_quad_indices() {
        let source = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nusemtl red paint\nf -4 -3 -2 -1";
        let mesh = parse_obj(source, "newmtl red paint\nKd 1 0.5 0\nd 1\nTr 0\nillum 2").unwrap();
        assert_eq!(mesh.faces.len(), 2);
        assert!(mesh.faces.iter().all(|face| face.material == 1));
        assert_eq!(mesh.materials[1].color, [255, 128, 0, 255]);
        assert_eq!(mesh.materials[1].surface, "NONE");
    }

    #[test]
    fn accepts_valid_uvs_normals_comments_and_continuations() {
        let source = "\u{feff}v 0 0 0\nv 2 0 0 2\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nf 1/1/1 2/2/1 \\\n3/3/1 # triangle\n";
        let mesh = parse_obj(source, "").unwrap();
        assert_eq!(mesh.vertices[1], [1.0, 0.0, 0.0]);
        assert_eq!(mesh.faces[0].indices, [0, 1, 2]);
        assert!(parse_obj(&source.replace("1/1/1 2/2/1", "1//-1 2//-1"), "").is_ok());
    }

    #[test]
    fn rejects_missing_zero_and_overflowing_indices() {
        for face in [
            "f 0 2 3",
            "f 1 2 4",
            "f -4 -2 -1",
            "f 1/1 2/1 3/1",
            "f 1//1 2//1 3//1",
            "f 1/ 2 3",
            "f 1/1/ 2 3",
            "f 9223372036854775808 2 3",
        ] {
            assert!(error(&TRIANGLE.replace("f 1 2 3", face), "").contains("OBJ line 4"));
        }
    }

    #[test]
    fn rejects_nonfinite_empty_and_degenerate_geometry() {
        assert!(error("", "").contains("vertices"));
        assert!(error(&TRIANGLE.replace("v 1 0 0", "v NaN 0 0"), "").contains("finite"));
        assert!(error(&TRIANGLE.replace("v 1 0 0", "v inf 0 0"), "").contains("finite"));
        assert!(error(&TRIANGLE.replace("v 1 0 0", "v 1000001 0 0"), "")
            .contains("within +/-1,000,000"));
        assert!(error(&TRIANGLE.replace("f 1 2 3", "f 1 1 3"), "").contains("repeats"));
        assert!(error(&TRIANGLE.replace("v 0 1 0", "v 2 0 0"), "").contains("degenerate"));
        assert!(error(&TRIANGLE.replace("v 1 0 0", "v 1 0 0 0"), "").contains("zero"));
    }

    #[test]
    fn discards_zero_area_exporter_triangles_without_losing_valid_faces() {
        let source = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 2 0 0\nf 1 2 4\nf 1 2 3";
        let mesh = parse_obj(source, "").unwrap();
        assert_eq!(mesh.faces.len(), 1);
        assert_eq!(mesh.faces[0].indices, [0, 1, 2]);
    }

    #[test]
    fn rejects_textures_transparency_and_unknown_materials() {
        for property in [
            "map_Kd albedo.png",
            "map_d alpha.png",
            "bump normal.png",
            "d 0.5",
            "Tr 1",
            "illum 4",
        ] {
            let material = format!("newmtl paint\nKd 1 0 0\n{property}");
            let message = error(TRIANGLE, &material);
            assert!(message.contains("unsupported"), "{message}");
        }
        assert!(
            error(&TRIANGLE.replace("f 1", "usemtl absent\nf 1"), "").contains("unknown material")
        );
        assert!(error(TRIANGLE, "newmtl paint\nKd 2 0 0").contains("between 0 and 1"));
        assert!(error(TRIANGLE, "newmtl paint\nKd NaN 0 0").contains("finite"));
        assert!(error(TRIANGLE, "newmtl paint\nnewmtl paint").contains("unique"));
    }

    fn signed_area(mesh: &PropMesh) -> f64 {
        mesh.faces
            .iter()
            .map(|face| {
                let [a, b, c] = face.indices.map(|i| {
                    [
                        f64::from(mesh.vertices[i][0]),
                        f64::from(mesh.vertices[i][1]),
                    ]
                });
                cross(a, b, c) / 2.0
            })
            .sum()
    }

    #[test]
    fn triangulates_concave_polygons_preserving_winding_and_area() {
        let vertices = "v 0 0 0\nv 2 0 0\nv 2 2 0\nv 1 1 0\nv 0 2 0\n";
        let mesh = parse_obj(&format!("{vertices}f 1 2 3 4 5"), "").unwrap();
        assert_eq!(mesh.faces.len(), 3);
        assert!((signed_area(&mesh) - 3.0).abs() < 1e-9);
        let reversed = parse_obj(&format!("{vertices}f 5 4 3 2 1"), "").unwrap();
        assert!((signed_area(&reversed) + 3.0).abs() < 1e-9);
    }

    #[test]
    fn removes_collinear_polygon_corners_without_zero_area_triangles() {
        let mesh = parse_obj(
            "v 0 0 0\nv 1 0 0\nv 2 0 0\nv 2 1 0\nv 0 1 0\nf 1 2 3 4 5",
            "",
        )
        .unwrap();
        assert!((signed_area(&mesh) - 2.0).abs() < 1e-9);
        for face in &mesh.faces {
            let [a, b, c] = face.indices.map(|i| {
                [
                    f64::from(mesh.vertices[i][0]),
                    f64::from(mesh.vertices[i][1]),
                ]
            });
            assert!(cross(a, b, c) > 0.0);
        }
    }

    #[test]
    fn rejects_crossing_and_nonplanar_polygons() {
        assert!(error("v 0 0 0\nv 1 1 0\nv 0 1 0\nv 1 0 0\nf 1 2 3 4", "")
            .contains("self-intersecting"));
        assert!(error("v 0 0 0\nv 1 0 0\nv 1 1 1\nv 0 1 0\nf 1 2 3 4", "").contains("non-planar"));
    }

    #[test]
    fn loads_material_libraries_relative_to_obj_path_and_only_once() {
        let directory = tempfile::tempdir().unwrap();
        let obj = directory.path().join("prop.obj");
        fs::write(
            &obj,
            format!(
                "mtllib prop colors.mtl\nmtllib \"prop colors.mtl\"\n{}",
                TRIANGLE.replace("f 1", "usemtl gold\nf 1")
            ),
        )
        .unwrap();
        fs::write(
            directory.path().join("prop colors.mtl"),
            "newmtl gold\nKd 1 0.8 0",
        )
        .unwrap();
        let mesh = load_obj(&obj).unwrap();
        assert_eq!(mesh.materials.len(), 2);
        assert_eq!(
            mesh.materials[mesh.faces[0].material].color,
            [255, 204, 0, 255]
        );
    }

    #[test]
    fn prevents_external_material_paths_and_reports_missing_libraries() {
        let directory = tempfile::tempdir().unwrap();
        let obj = directory.path().join("prop.obj");
        for name in ["../outside.mtl", "/etc/passwd", "C:\\outside.mtl"] {
            fs::write(&obj, format!("mtllib {name}\n{TRIANGLE}")).unwrap();
            let result = load_obj(&obj);
            assert!(
                matches!(result, Err(PropError::Invalid(message)) if message.contains("inside the OBJ directory"))
            );
        }
        fs::write(&obj, format!("mtllib missing.mtl\n{TRIANGLE}")).unwrap();
        assert!(
            matches!(load_obj(&obj), Err(PropError::Invalid(message)) if message.contains("cannot open material"))
        );
    }
}
