use godot::prelude::*;
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};

struct InfernalExtension;

#[gdextension]
unsafe impl ExtensionLibrary for InfernalExtension {}

struct Job {
    id: i64,
    prompt: String,
    seed: u64,
    output_dir: Option<PathBuf>,
    library_dir: PathBuf,
    command: Command,
}

#[derive(Clone, Copy)]
enum Command {
    Generate,
    Suggest(u32),
    SuggestArena(u32),
    Save(i64),
    Release(i64),
}

struct JobResult {
    id: i64,
    prompt: String,
    difficulty: u32,
    monster_seed: u64,
    path: String,
    backend: String,
    error: String,
    packages: Vec<MemoryPackage>,
}

struct RawImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
struct MemoryPackage {
    path: String,
    monster: serde_json::Value,
    sprites: RawImage,
    emission: RawImage,
    projectiles: Option<RawImage>,
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct InfernalGenerator {
    jobs: SyncSender<Job>,
    results: Receiver<JobResult>,
    next_id: i64,
}

#[godot_api]
impl IRefCounted for InfernalGenerator {
    fn init(_base: Base<RefCounted>) -> Self {
        let (jobs, incoming) = sync_channel(1);
        let (outgoing, results) = sync_channel(2);
        std::thread::Builder::new()
            .name("infernal-generator".into())
            .spawn(move || worker(incoming, outgoing))
            .expect("failed to start Infernal Diffusion worker");
        Self {
            jobs,
            results,
            next_id: 1,
        }
    }
}

#[godot_api]
impl InfernalGenerator {
    /// All paths must be absolute OS paths. Call ProjectSettings.globalize_path()
    /// before this method; no Godot API is used from the worker thread.
    #[func]
    fn generate_async(
        &mut self,
        prompt: GString,
        seed: i64,
        output_dir: GString,
        library_dir: GString,
    ) -> i64 {
        let path = PathBuf::from(output_dir.to_string());
        let libraries = PathBuf::from(library_dir.to_string());
        if !path.is_absolute() || !libraries.is_absolute() || seed < 0 {
            return -1;
        }
        let id = self.next_id;
        let job = Job {
            id,
            prompt: prompt.to_string(),
            seed: seed as u64,
            output_dir: Some(path),
            library_dir: libraries,
            command: Command::Generate,
        };
        match self.jobs.try_send(job) {
            Ok(()) => {
                self.next_id += 1;
                id
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => -1,
        }
    }

    /// Generate native Rust monster objects and raw RGBA atlases entirely in memory.
    #[func]
    fn generate_in_memory_async(
        &mut self,
        prompt: GString,
        seed: i64,
        library_dir: GString,
    ) -> i64 {
        let libraries = PathBuf::from(library_dir.to_string());
        if !libraries.is_absolute() || seed < 0 {
            return -1;
        }
        let id = self.next_id;
        let job = Job {
            id,
            prompt: prompt.to_string(),
            seed: seed as u64,
            output_dir: None,
            library_dir: libraries,
            command: Command::Generate,
        };
        match self.jobs.try_send(job) {
            Ok(()) => {
                self.next_id += 1;
                id
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => -1,
        }
    }

    /// Persist a previously generated in-memory object without regenerating it.
    #[func]
    fn save_in_memory_async(&mut self, object_job_id: i64, output_dir: GString) -> i64 {
        let path = PathBuf::from(output_dir.to_string());
        if !path.is_absolute() {
            return -1;
        }
        let id = self.next_id;
        let job = Job {
            id,
            prompt: String::new(),
            seed: 0,
            output_dir: Some(path),
            library_dir: PathBuf::new(),
            command: Command::Save(object_job_id),
        };
        match self.jobs.try_send(job) {
            Ok(()) => {
                self.next_id += 1;
                id
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => -1,
        }
    }

    #[func]
    fn release_in_memory(&mut self, object_job_id: i64) -> bool {
        self.jobs
            .try_send(Job {
                id: 0,
                prompt: String::new(),
                seed: 0,
                output_dir: None,
                library_dir: PathBuf::new(),
                command: Command::Release(object_job_id),
            })
            .is_ok()
    }

    /// Queue a cheap recipe-aware prompt for an NPC to speak before spawning.
    #[func]
    fn suggest_prompt_async(&mut self, seed: i64, difficulty: i32, library_dir: GString) -> i64 {
        self.queue_suggestion(seed, difficulty, library_dir, false)
    }

    /// Queue a one-based arena round. Difficulty rises every three rounds.
    #[func]
    fn suggest_arena_prompt_async(&mut self, run_seed: i64, round: i32, library_dir: GString) -> i64 {
        self.queue_suggestion(run_seed, round, library_dir, true)
    }

    fn queue_suggestion(&mut self, seed: i64, value: i32, library_dir: GString, arena: bool) -> i64 {
        let libraries = PathBuf::from(library_dir.to_string());
        if seed < 0 || value < 1 || !libraries.is_absolute() || (!arena && value > 3) {
            return -1;
        }
        let id = self.next_id;
        let job = Job {
            id,
            prompt: String::new(),
            seed: seed as u64,
            output_dir: None,
            library_dir: libraries,
            command: if arena { Command::SuggestArena(value as u32) } else { Command::Suggest(value as u32) },
        };
        match self.jobs.try_send(job) {
            Ok(()) => { self.next_id += 1; id }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => -1,
        }
    }

    /// Poll once per frame. Empty Dictionary means no completed job.
    #[func]
    fn poll_result(&mut self) -> VarDictionary {
        match self.results.try_recv() {
            Ok(result) => {
                let mut value = VarDictionary::new();
                value.set("job_id", result.id);
                value.set("prompt", result.prompt);
                value.set("difficulty", result.difficulty);
                value.set("monster_seed", result.monster_seed as i64);
                value.set("ok", result.error.is_empty());
                value.set("package_dir", result.path);
                value.set("backend", result.backend);
                value.set("error", result.error);
                if !result.packages.is_empty() {
                    let mut packages = VarArray::new();
                    for package in result.packages {
                        let mut entry = VarDictionary::new();
                        entry.set("path", package.path);
                        entry.set("monster", &to_godot_value(&package.monster));
                        entry.set("sprites", &image_to_dictionary(package.sprites));
                        entry.set("emission", &image_to_dictionary(package.emission));
                        if let Some(projectiles) = package.projectiles {
                            entry.set("projectiles", &image_to_dictionary(projectiles));
                        }
                        packages.push(&entry.to_variant());
                    }
                    value.set("packages", &packages);
                }
                value
            }
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => VarDictionary::new(),
        }
    }
}

fn to_godot_value(value: &serde_json::Value) -> Variant {
    match value {
        serde_json::Value::Null => Variant::nil(),
        serde_json::Value::Bool(value) => value.to_variant(),
        serde_json::Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                integer.to_variant()
            } else {
                value.as_f64().unwrap_or(0.0).to_variant()
            }
        }
        serde_json::Value::String(value) => value.to_variant(),
        serde_json::Value::Array(values) => {
            let mut array = VarArray::new();
            for value in values {
                array.push(&to_godot_value(value));
            }
            array.to_variant()
        }
        serde_json::Value::Object(values) => {
            let mut dictionary = VarDictionary::new();
            for (key, value) in values {
                dictionary.set(key.as_str(), &to_godot_value(value));
            }
            dictionary.to_variant()
        }
    }
}

fn image_to_dictionary(image: RawImage) -> VarDictionary {
    let mut value = VarDictionary::new();
    value.set("width", image.width);
    value.set("height", image.height);
    value.set("rgba", &PackedByteArray::from(image.rgba));
    value
}

fn worker(incoming: Receiver<Job>, outgoing: SyncSender<JobResult>) {
    let mut backend: Option<Backend> = None;
    let mut objects = std::collections::HashMap::<i64, *mut c_void>::new();
    while let Ok(job) = incoming.recv() {
        if let Command::Release(source) = job.command {
            if let (Some(active), Some(object)) = (backend.as_ref(), objects.remove(&source)) {
                unsafe { (active.free_object)(object) };
            }
            continue;
        }
        let result = match job.command {
            Command::Save(source) => save_object(&job, source, backend.as_ref(), &objects),
            Command::Generate if job.output_dir.is_none() && objects.len() >= 4 => JobResult {
                id: job.id,
                prompt: job.prompt.clone(),
                difficulty: 0,
                monster_seed: job.seed,
                path: String::new(),
                backend: String::new(),
                error: "release an in-memory monster before generating another".into(),
                packages: Vec::new(),
            },
            Command::Generate | Command::Suggest(_) | Command::SuggestArena(_) => {
                run_job(&job, &mut backend, &mut objects)
            }
            Command::Release(_) => unreachable!(),
        };
        if outgoing.send(result).is_err() {
            break;
        }
    }
    if let Some(active) = backend.as_ref() {
        for object in objects.into_values() {
            unsafe { (active.free_object)(object) };
        }
    }
}

fn save_object(
    job: &Job,
    source: i64,
    backend: Option<&Backend>,
    objects: &std::collections::HashMap<i64, *mut c_void>,
) -> JobResult {
    let mut result = JobResult {
        id: job.id,
        prompt: String::new(),
        difficulty: 0,
        monster_seed: 0,
        path: job
            .output_dir
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        backend: String::new(),
        error: String::new(),
        packages: Vec::new(),
    };
    let (Some(active), Some(&object)) = (backend, objects.get(&source)) else {
        result.error = "in-memory monster id not found".into();
        return result;
    };
    result.backend = active.kernel_name();
    let Ok(path) = CString::new(result.path.as_str()) else {
        result.error = "output path contains a NUL byte".into();
        return result;
    };
    let mut error_ptr = std::ptr::null_mut();
    let code = unsafe { (active.object_save)(object, path.as_ptr(), &mut error_ptr) };
    let message = copy_error(error_ptr, active.free);
    if code != 0 {
        result.error = if message.is_empty() {
            "save failed".into()
        } else {
            message
        };
    }
    result
}

type Generate = unsafe extern "C" fn(*const c_char, u64, *const c_char, *mut *mut c_char) -> i32;
type FreeString = unsafe extern "C" fn(*mut c_char);
type AbiVersion = unsafe extern "C" fn() -> u32;
type CpuKernelName = unsafe extern "C" fn() -> *const c_char;
type RandomPrompt = unsafe extern "C" fn(u64, u32, *mut *mut c_char) -> *mut c_char;
type ArenaPrompt = unsafe extern "C" fn(u64, u32, *mut u32, *mut u64, *mut *mut c_char) -> *mut c_char;
type GenerateObject = unsafe extern "C" fn(*const c_char, u64, *mut *mut c_char) -> *mut c_void;
type FreeObject = unsafe extern "C" fn(*mut c_void);
type ObjectCount = unsafe extern "C" fn(*const c_void) -> u32;
type ObjectPath = unsafe extern "C" fn(*const c_void, u32) -> *mut c_char;
type ObjectPixels = unsafe extern "C" fn(
    *const c_void,
    u32,
    u32,
    *mut *const u8,
    *mut usize,
    *mut u32,
    *mut u32,
) -> i32;
type VisitCallback = unsafe extern "C" fn(*mut c_void, u32, *const c_char, *const c_char, i64, f64);
type ObjectVisit =
    unsafe extern "C" fn(*const c_void, u32, *mut c_void, Option<VisitCallback>) -> i32;
type ObjectSave = unsafe extern "C" fn(*const c_void, *const c_char, *mut *mut c_char) -> i32;

struct Backend {
    path: PathBuf,
    cpu_kernel_name: CpuKernelName,
    random_prompt: RandomPrompt,
    arena_prompt: ArenaPrompt,
    _library: libloading::Library,
    generate: Generate,
    free: FreeString,
    generate_object: GenerateObject,
    free_object: FreeObject,
    object_count: ObjectCount,
    object_path: ObjectPath,
    object_pixels: ObjectPixels,
    object_visit: ObjectVisit,
    object_save: ObjectSave,
}

impl Backend {
    fn kernel_name(&self) -> String {
        let pointer = unsafe { (self.cpu_kernel_name)() };
        if pointer.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned()
    }
}

fn load_backend(library_dir: &PathBuf) -> Result<Backend, String> {
    let mut last_error = String::new();
    for tier in tiers() {
        let path = library_dir.join(core_name(tier));
        if !path.is_file() {
            continue;
        }
        let library = match unsafe { libloading::Library::new(&path) } {
            Ok(value) => value,
            Err(error) => {
                last_error = format!("{}: {error}", path.display());
                continue;
            }
        };
        let functions = unsafe {
            let abi: AbiVersion = match library.get(b"infernal_abi_version") {
                Ok(value) => *value,
                Err(error) => {
                    last_error = error.to_string();
                    continue;
                }
            };
            if abi() != 1 {
                last_error = "unsupported generator ABI".into();
                continue;
            }
            let generate: Generate = match library.get(b"infernal_generate_3d") {
                Ok(value) => *value,
                Err(error) => {
                    last_error = error.to_string();
                    continue;
                }
            };
            let free: FreeString = match library.get(b"infernal_free_string") {
                Ok(value) => *value,
                Err(error) => {
                    last_error = error.to_string();
                    continue;
                }
            };
            macro_rules! symbol {
                ($name:literal, $type:ty) => {
                    match library.get::<$type>($name) {
                        Ok(value) => *value,
                        Err(error) => {
                            last_error = error.to_string();
                            continue;
                        }
                    }
                };
            }
            (
                generate,
                free,
                symbol!(b"infernal_cpu_kernel_name", CpuKernelName),
                symbol!(b"infernal_random_prompt", RandomPrompt),
                symbol!(b"infernal_arena_prompt", ArenaPrompt),
                symbol!(b"infernal_generate_object", GenerateObject),
                symbol!(b"infernal_object_free", FreeObject),
                symbol!(b"infernal_object_count", ObjectCount),
                symbol!(b"infernal_object_package_path", ObjectPath),
                symbol!(b"infernal_object_pixels", ObjectPixels),
                symbol!(b"infernal_object_visit", ObjectVisit),
                symbol!(b"infernal_object_save", ObjectSave),
            )
        };
        return Ok(Backend {
            path,
            cpu_kernel_name: functions.2,
            random_prompt: functions.3,
            arena_prompt: functions.4,
            _library: library,
            generate: functions.0,
            free: functions.1,
            generate_object: functions.5,
            free_object: functions.6,
            object_count: functions.7,
            object_path: functions.8,
            object_pixels: functions.9,
            object_visit: functions.10,
            object_save: functions.11,
        });
    }
    if last_error.is_empty() {
        Err(format!(
            "no compatible generator library in {}",
            library_dir.display()
        ))
    } else {
        Err(last_error)
    }
}

fn run_job(
    job: &Job,
    backend: &mut Option<Backend>,
    objects: &mut std::collections::HashMap<i64, *mut c_void>,
) -> JobResult {
    let mut result = JobResult {
        id: job.id,
        prompt: job.prompt.clone(),
        difficulty: 0,
        monster_seed: job.seed,
        path: job
            .output_dir
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        backend: String::new(),
        error: String::new(),
        packages: Vec::new(),
    };
    let prompt = match CString::new(job.prompt.as_str()) {
        Ok(value) => value,
        Err(_) => {
            result.error = "prompt contains a NUL byte".into();
            return result;
        }
    };
    if backend
        .as_ref()
        .is_some_and(|value| value.path.parent() != Some(job.library_dir.as_path()))
    {
        result.error = "library directory cannot change for this generator".into();
        return result;
    }
    if backend.is_none() {
        *backend = match load_backend(&job.library_dir) {
            Ok(value) => Some(value),
            Err(error) => {
                result.error = error;
                return result;
            }
        };
    }
    let active = backend.as_ref().expect("loaded above");
    result.backend = active.kernel_name();
    match job.command {
        Command::Suggest(difficulty) => {
            let mut error_ptr = std::ptr::null_mut();
            let pointer = unsafe { (active.random_prompt)(job.seed, difficulty, &mut error_ptr) };
            if pointer.is_null() {
                result.error = copy_error(error_ptr, active.free);
                if result.error.is_empty() {
                    result.error = "prompt composition failed".into();
                }
            } else {
                result.prompt = copy_error(pointer, active.free);
                result.difficulty = difficulty;
            }
            return result;
        }
        Command::SuggestArena(round) => {
            let mut difficulty = 0;
            let mut monster_seed = 0;
            let mut error_ptr = std::ptr::null_mut();
            let pointer = unsafe {
                (active.arena_prompt)(job.seed, round, &mut difficulty, &mut monster_seed, &mut error_ptr)
            };
            if pointer.is_null() {
                result.error = copy_error(error_ptr, active.free);
                if result.error.is_empty() {
                    result.error = "arena prompt composition failed".into();
                }
            } else {
                result.prompt = copy_error(pointer, active.free);
                result.difficulty = difficulty;
                result.monster_seed = monster_seed;
            }
            return result;
        }
        Command::Generate => {}
        Command::Save(_) | Command::Release(_) => unreachable!(),
    }
    if job.output_dir.is_none() {
        let mut error_ptr: *mut c_char = std::ptr::null_mut();
        let object = unsafe { (active.generate_object)(prompt.as_ptr(), job.seed, &mut error_ptr) };
        if object.is_null() {
            result.error = copy_error(error_ptr, active.free);
            if result.error.is_empty() {
                result.error = "generation failed".into();
            }
            return result;
        }
        result.packages = unsafe { read_packages(active, object) }.unwrap_or_else(|error| {
            result.error = error;
            Vec::new()
        });
        if result.error.is_empty() {
            objects.insert(job.id, object);
        } else {
            unsafe { (active.free_object)(object) };
        }
        return result;
    }
    let output = match CString::new(result.path.as_str()) {
        Ok(value) => value,
        Err(_) => {
            result.error = "output path contains a NUL byte".into();
            return result;
        }
    };
    let mut error_ptr: *mut c_char = std::ptr::null_mut();
    let code =
        unsafe { (active.generate)(prompt.as_ptr(), job.seed, output.as_ptr(), &mut error_ptr) };
    let message = copy_error(error_ptr, active.free);
    if code != 0 {
        result.error = if message.is_empty() {
            "generation failed".into()
        } else {
            message
        };
    }
    result
}

fn copy_error(pointer: *mut c_char, free: FreeString) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let value = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    unsafe { free(pointer) };
    value
}

struct TreeBuilder {
    stack: Vec<(Option<String>, serde_json::Value)>,
    root: Option<serde_json::Value>,
}
impl TreeBuilder {
    fn add(&mut self, key: Option<String>, value: serde_json::Value) {
        if let Some((_, parent)) = self.stack.last_mut() {
            match parent {
                serde_json::Value::Array(items) => items.push(value),
                serde_json::Value::Object(fields) => {
                    if let Some(key) = key {
                        fields.insert(key, value);
                    }
                }
                _ => {}
            }
        } else {
            self.root = Some(value);
        }
    }
}

unsafe extern "C" fn collect_value(
    context: *mut c_void,
    kind: u32,
    key: *const c_char,
    string: *const c_char,
    integer: i64,
    real: f64,
) {
    if context.is_null() {
        return;
    }
    let builder = &mut *(context as *mut TreeBuilder);
    let key = if key.is_null() {
        None
    } else {
        Some(CStr::from_ptr(key).to_string_lossy().into_owned())
    };
    match kind {
        1 => builder
            .stack
            .push((key, serde_json::Value::Object(serde_json::Map::new()))),
        3 => builder
            .stack
            .push((key, serde_json::Value::Array(Vec::new()))),
        2 | 4 => {
            if let Some((name, value)) = builder.stack.pop() {
                builder.add(name, value);
            }
        }
        0 => builder.add(key, serde_json::Value::Null),
        5 => {
            if !string.is_null() {
                builder.add(
                    key,
                    serde_json::Value::String(
                        CStr::from_ptr(string).to_string_lossy().into_owned(),
                    ),
                );
            }
        }
        6 => builder.add(key, serde_json::Value::from(integer)),
        7 => builder.add(key, serde_json::Value::from(real)),
        8 => builder.add(key, serde_json::Value::from(integer != 0)),
        _ => {}
    }
}

unsafe fn read_image(
    active: &Backend,
    object: *const c_void,
    index: u32,
    kind: u32,
) -> Option<RawImage> {
    let mut ptr: *const u8 = std::ptr::null();
    let mut len = 0usize;
    let mut width = 0u32;
    let mut height = 0u32;
    if (active.object_pixels)(
        object,
        index,
        kind,
        &mut ptr,
        &mut len,
        &mut width,
        &mut height,
    ) != 0
        || ptr.is_null()
    {
        return None;
    }
    if len != width as usize * height as usize * 4 {
        return None;
    }
    Some(RawImage {
        width,
        height,
        rgba: std::slice::from_raw_parts(ptr, len).to_vec(),
    })
}

unsafe fn read_packages(
    active: &Backend,
    object: *const c_void,
) -> Result<Vec<MemoryPackage>, String> {
    let count = (active.object_count)(object);
    if count == 0 || count > 16 {
        return Err("invalid in-memory package count".into());
    }
    let mut packages = Vec::with_capacity(count as usize);
    for index in 0..count {
        let path_ptr = (active.object_path)(object, index);
        if path_ptr.is_null() {
            return Err("missing package path".into());
        }
        let path = CStr::from_ptr(path_ptr).to_string_lossy().into_owned();
        (active.free)(path_ptr);
        let mut builder = TreeBuilder {
            stack: Vec::new(),
            root: None,
        };
        if (active.object_visit)(
            object,
            index,
            &mut builder as *mut _ as *mut c_void,
            Some(collect_value),
        ) != 0
        {
            return Err("invalid monster object".into());
        }
        let monster = builder.root.ok_or("missing monster data")?;
        let sprites = read_image(active, object, index, 0).ok_or("missing sprite data")?;
        let emission = read_image(active, object, index, 1).ok_or("missing emission data")?;
        let projectiles = read_image(active, object, index, 2);
        packages.push(MemoryPackage {
            path,
            monster,
            sprites,
            emission,
            projectiles,
        });
    }
    Ok(packages)
}

fn core_name(tier: &str) -> String {
    let prefix = if cfg!(target_os = "windows") {
        ""
    } else {
        "lib"
    };
    let suffix = if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    format!("{prefix}infernal_diffusion_{tier}.{suffix}")
}

fn tiers() -> Vec<&'static str> {
    #[cfg(target_arch = "x86_64")]
    {
        vec!["x64"]
    }
    #[cfg(target_arch = "x86")]
    {
        vec!["x86"]
    }
    #[cfg(target_arch = "aarch64")]
    {
        vec!["arm64"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_library_names_match_platform() {
        assert!(core_name("x64").contains("infernal_diffusion_x64"));
        assert!(tiers().contains(&"x64") || tiers().contains(&"x86") || tiers().contains(&"arm64"));
    }
}
