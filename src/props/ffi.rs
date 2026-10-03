//! Additive C ABI for static prop baking. Prop handles never contain monsters.
use super::{bake, load_mesh, PropAtlas, PropOptions};
use std::ffi::{c_char, CStr, CString};
use std::path::Path;

/// Opaque prop atlas owned by the caller until `infernal_prop_free`.
pub struct InfernalPropAtlas {
    atlas: PropAtlas,
    metadata_json: CString,
}

unsafe fn set_error(error_out: *mut *mut c_char, message: String) {
    if !error_out.is_null() {
        *error_out = CString::new(message.replace('\0', "\\0"))
            .unwrap_or_default()
            .into_raw();
    }
}

/// Load an OBJ or prop JSON mesh and render its static directional atlas.
/// Null `options_json` uses defaults; otherwise it must be a JSON object.
/// Returns null on error. Free errors with `infernal_free_string`.
///
/// # Safety
/// Input strings must be NUL-terminated UTF-8. `error_out`, if non-null, must
/// point to writable pointer storage. Returned handles must be freed once.
#[no_mangle]
pub unsafe extern "C" fn infernal_bake_prop(
    mesh_path: *const c_char,
    options_json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut InfernalPropAtlas {
    if !error_out.is_null() {
        *error_out = std::ptr::null_mut();
    }
    let result = std::panic::catch_unwind(|| {
        if mesh_path.is_null() {
            return Err("null prop mesh path".to_string());
        }
        let path = CStr::from_ptr(mesh_path)
            .to_str()
            .map_err(|error| format!("invalid UTF-8 prop path: {error}"))?;
        let options = if options_json.is_null() {
            PropOptions::default()
        } else {
            let text = CStr::from_ptr(options_json)
                .to_str()
                .map_err(|error| format!("invalid UTF-8 prop options: {error}"))?;
            serde_json::from_str::<PropOptions>(text)
                .map_err(|error| format!("invalid prop options JSON: {error}"))?
        };
        let mesh = load_mesh(Path::new(path)).map_err(|error| error.to_string())?;
        let atlas = bake(&mesh, &options).map_err(|error| error.to_string())?;
        let metadata_json = serde_json::to_string(&atlas.metadata)
            .map_err(|error| format!("cannot serialize prop metadata: {error}"))?;
        let metadata_json = CString::new(metadata_json)
            .map_err(|error| format!("invalid prop metadata: {error}"))?;
        Ok(InfernalPropAtlas {
            atlas,
            metadata_json,
        })
    });
    match result {
        Ok(Ok(value)) => Box::into_raw(Box::new(value)),
        Ok(Err(message)) => {
            set_error(error_out, message);
            std::ptr::null_mut()
        }
        Err(_) => {
            set_error(error_out, "panic while baking prop".into());
            std::ptr::null_mut()
        }
    }
}

/// # Safety
/// `handle` must be null or a live `infernal_bake_prop` result and freed once.
#[no_mangle]
pub unsafe extern "C" fn infernal_prop_free(handle: *mut InfernalPropAtlas) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Returns tightly packed RGBA8 data borrowed until `infernal_prop_free`.
/// Returns -1 for null arguments, clearing all valid output pointers first.
///
/// # Safety
/// `handle` must be null or a live prop handle. Non-null output pointers must
/// point to writable storage. Pixels must not be mutated or freed by callers.
#[no_mangle]
pub unsafe extern "C" fn infernal_prop_pixels(
    handle: *const InfernalPropAtlas,
    pixels_out: *mut *const u8,
    len_out: *mut usize,
    width_out: *mut u32,
    height_out: *mut u32,
) -> i32 {
    if !pixels_out.is_null() {
        *pixels_out = std::ptr::null();
    }
    if !len_out.is_null() {
        *len_out = 0;
    }
    if !width_out.is_null() {
        *width_out = 0;
    }
    if !height_out.is_null() {
        *height_out = 0;
    }
    if handle.is_null()
        || pixels_out.is_null()
        || len_out.is_null()
        || width_out.is_null()
        || height_out.is_null()
    {
        return -1;
    }
    let image = &(*handle).atlas.sprites;
    *pixels_out = image.as_raw().as_ptr();
    *len_out = image.as_raw().len();
    *width_out = image.width();
    *height_out = image.height();
    0
}

/// Returns allocated UTF-8 JSON metadata, independent of the prop lifetime.
/// Returns null for a null handle. Free the string with `infernal_free_string`.
///
/// # Safety
/// `handle` must be null or a live `infernal_bake_prop` result.
#[no_mangle]
pub unsafe extern "C" fn infernal_prop_metadata_json(
    handle: *const InfernalPropAtlas,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    (*handle).metadata_json.clone().into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infernal_free_string;

    #[test]
    fn prop_ffi_reports_bad_inputs_without_panicking() {
        unsafe {
            let mut error = std::ptr::null_mut();
            assert!(infernal_bake_prop(std::ptr::null(), std::ptr::null(), &mut error).is_null());
            assert!(CStr::from_ptr(error)
                .to_str()
                .unwrap()
                .contains("null prop"));
            infernal_free_string(error);
            let path = CString::new("missing.obj").unwrap();
            let bad = CString::new("{bad json").unwrap();
            assert!(infernal_bake_prop(path.as_ptr(), bad.as_ptr(), &mut error).is_null());
            assert!(CStr::from_ptr(error)
                .to_str()
                .unwrap()
                .contains("options JSON"));
            infernal_free_string(error);
            assert!(
                infernal_bake_prop(path.as_ptr(), std::ptr::null(), std::ptr::null_mut()).is_null()
            );
            infernal_prop_free(std::ptr::null_mut());
            assert!(infernal_prop_metadata_json(std::ptr::null()).is_null());
            let mut pixels = std::ptr::dangling::<u8>();
            let (mut len, mut width, mut height) = (1, 1, 1);
            assert_eq!(
                infernal_prop_pixels(
                    std::ptr::null(),
                    &mut pixels,
                    &mut len,
                    &mut width,
                    &mut height
                ),
                -1
            );
            assert!(pixels.is_null());
            assert_eq!((len, width, height), (0, 0, 0));
        }
    }

    #[test]
    fn prop_ffi_copies_metadata_and_borrows_rgba() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tetrahedron.obj");
        std::fs::write(
            &path,
            "v -1 0 -1\nv 1 0 -1\nv 0 0 1\nv 0 2 0\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 3 1 4\n",
        )
        .unwrap();
        let path = CString::new(path.to_str().unwrap()).unwrap();
        unsafe {
            let mut error = std::ptr::null_mut();
            let handle = infernal_bake_prop(path.as_ptr(), std::ptr::null(), &mut error);
            assert!(
                !handle.is_null(),
                "{}",
                if error.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(error).to_string_lossy().into_owned()
                }
            );
            assert!(error.is_null());
            let mut pixels = std::ptr::null();
            let (mut len, mut width, mut height) = (0, 0, 0);
            assert_eq!(
                infernal_prop_pixels(handle, &mut pixels, &mut len, &mut width, &mut height),
                0
            );
            assert!(!pixels.is_null());
            assert!(width > 0 && height > 0);
            assert_eq!(len, width as usize * height as usize * 4);
            let copied_pixels = std::slice::from_raw_parts(pixels, len).to_vec();
            let metadata = infernal_prop_metadata_json(handle);
            assert!(!metadata.is_null());
            infernal_prop_free(handle);
            let value: serde_json::Value =
                serde_json::from_str(CStr::from_ptr(metadata).to_str().unwrap()).unwrap();
            assert!(value.is_object());
            assert!(value.get("behavior").is_none() && value.get("attacks").is_none());
            assert!(copied_pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] > 0));
            infernal_free_string(metadata);
        }
    }
}
