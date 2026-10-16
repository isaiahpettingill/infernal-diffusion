//! Admission limits for interactive, compact generation. These are conservative
//! estimates of generator-owned work, not an operating-system memory/CPU quota.
//! Offline/dense entry points retain their legacy behavior.
use crate::{anatomy::Body, parser::MonsterSpec, Error};

/// Generous defaults allow ordinary large monsters and companions while rejecting
/// runaway composites before dense raster buffers are allocated. Raising limits
/// does not override existing parser, geometry or package-format constraints.
#[derive(Clone, Copy, Debug)]
pub struct GenerationBudget {
    pub max_prompt_bytes: u64,
    /// Number of mount edges: three permits a four-creature stack.
    pub max_mount_depth: u64,
    /// Includes the root, detached rider/mount packages and summoned minion.
    pub max_packages: u64,
    pub max_heads_per_creature: u64,
    pub max_legs_per_creature: u64,
    pub max_arms_per_creature: u64,
    pub max_declared_parts: u64,
    pub max_body_nodes: u64,
    pub max_poses: u64,
    pub max_frame_dimension: u64,
    pub max_atlas_dimension: u64,
    pub max_dense_pixels: u64,
    /// Cumulative reservation across all packages, including retained parents.
    /// Includes dense/packed pixel temporaries, raster scratch and mesh allowance.
    /// Does not include the parser/model, allocator overhead or engine/JS copies.
    pub max_estimated_working_bytes: u64,
}

impl Default for GenerationBudget {
    fn default() -> Self {
        Self {
            max_prompt_bytes: 4096,
            max_mount_depth: 3,
            max_packages: 8,
            max_heads_per_creature: 5,
            max_legs_per_creature: 12,
            max_arms_per_creature: 8,
            max_declared_parts: 128,
            max_body_nodes: 384,
            max_poses: 512,
            max_frame_dimension: 512,
            max_atlas_dimension: 16384,
            max_dense_pixels: 64 * 1024 * 1024,
            max_estimated_working_bytes: 2 * 1024 * 1024 * 1024,
        }
    }
}

fn limit(resource: &'static str, requested: u64, maximum: u64) -> Result<(), Error> {
    if requested > maximum {
        Err(Error::ResourceLimit {
            resource,
            requested,
            limit: maximum,
        })
    } else {
        Ok(())
    }
}

/// Candle/gemm use the current Rayon pool. Confine bounded native parsing to
/// two workers (one on small hosts), rather than the process-global pool. A
/// failed pool initialization must not silently fall back to all-core parsing.
/// The embedded model remains cached on these stable pool threads.
pub(crate) fn with_parser_pool<R: Send>(
    parse: impl FnOnce() -> Result<R, Error> + Send,
) -> Result<R, Error> {
    #[cfg(all(feature = "bert", not(target_arch = "wasm32")))]
    {
        static POOL: std::sync::OnceLock<Result<rayon::ThreadPool, String>> =
            std::sync::OnceLock::new();
        let pool = POOL.get_or_init(|| {
            let cores = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);
            rayon::ThreadPoolBuilder::new()
                .num_threads(if cores < 4 { 1 } else { 2 })
                .thread_name(|i| format!("infernal-parser-{i}"))
                .build()
                .map_err(|e| e.to_string())
        });
        match pool {
            Ok(pool) => pool.install(parse),
            Err(error) => Err(Error::Model(format!(
                "bounded parser pool unavailable: {error}"
            ))),
        }
    }
    #[cfg(any(not(feature = "bert"), target_arch = "wasm32"))]
    parse()
}

pub(crate) struct BudgetState {
    limits: GenerationBudget,
    packages: u64,
    reserved_bytes: u64,
}

impl BudgetState {
    pub(crate) fn new(limits: GenerationBudget) -> Self {
        Self {
            limits,
            packages: 0,
            reserved_bytes: 0,
        }
    }

    pub(crate) fn check_prompt(&self, prompt: &str) -> Result<(), Error> {
        limit(
            "prompt bytes",
            prompt.len() as u64,
            self.limits.max_prompt_bytes,
        )?;
        // Match the parser's delimiter exactly. Its four-clause compatibility
        // parser truncates longer chains; interactive requests must fail instead.
        let depth = prompt.to_lowercase().matches(" riding ").count() as u64;
        limit("mount depth", depth, self.limits.max_mount_depth.min(3))
    }

    pub(crate) fn check_spec(&self, spec: &MonsterSpec) -> Result<(), Error> {
        let mut current = spec;
        let mut depth = 0;
        let mut parts = 0u64;
        loop {
            limit(
                "heads per creature",
                current.heads.into(),
                self.limits.max_heads_per_creature,
            )?;
            limit(
                "legs per creature",
                current.limb_count.into(),
                self.limits.max_legs_per_creature,
            )?;
            limit(
                "arms per creature",
                current.arm_count.into(),
                self.limits.max_arms_per_creature,
            )?;
            // Protect the loops driven by direct specs as well as parsed prompts.
            limit("features per creature", current.features.len() as u64, 128)?;
            limit("materials per creature", current.materials.len() as u64, 32)?;
            parts = parts.saturating_add(
                u64::from(current.heads)
                    + u64::from(current.limb_count)
                    + u64::from(current.arm_count),
            );
            limit(
                "declared anatomy parts",
                parts,
                self.limits.max_declared_parts,
            )?;
            if let Some(spawn) = &current.spawn {
                // Generation reparses non-SELF minion targets before removing
                // their mount/spawn fields; apply the input guard beforehand.
                self.check_prompt(&spawn.target.to_lowercase().replace('_', " "))?;
            }
            let Some(mount) = &current.mount else { break };
            depth += 1;
            limit("mount depth", depth, self.limits.max_mount_depth)?;
            let Some(child) = mount.spec.as_deref() else {
                break;
            };
            current = child;
        }
        // Each mount edge adds the detached rider and mount; nested spawns are
        // deliberately disabled by the existing companion-generation contract.
        let packages = 1 + depth * 2 + u64::from(spec.spawn.is_some());
        limit("package count", packages, self.limits.max_packages)
    }

    pub(crate) fn begin_package(&mut self) -> Result<(), Error> {
        let count = self.packages.saturating_add(1);
        limit("package count", count, self.limits.max_packages)?;
        self.packages = count;
        Ok(())
    }

    pub(crate) fn check_body(&self, body: &Body) -> Result<(), Error> {
        limit(
            "anatomy nodes",
            body.nodes.len() as u64,
            self.limits.max_body_nodes,
        )
    }

    pub(crate) fn check_render(&self, size: u32, poses: usize, nodes: usize) -> Result<u64, Error> {
        let budget = &self.limits;
        limit("animation poses", poses as u64, budget.max_poses)?;
        limit("frame dimension", size.into(), budget.max_frame_dimension)?;
        // Match render3d::sheet's layout. Check all arithmetic in u64 before its
        // u32 conversions and ImageBuffer allocations (also on wasm32).
        let views = crate::render3d::ANGLES_DEG.len() as u64;
        let total = (poses as u64).saturating_mul(views);
        limit("directional frames", total, u32::MAX.into())?;
        let columns = ((total as f32).sqrt().ceil() as u64).max(1);
        let rows = total.div_ceil(columns);
        let width = columns.saturating_mul(size.into());
        let height = rows.saturating_mul(size.into());
        limit(
            "dense atlas width",
            width,
            budget.max_atlas_dimension.min(u32::MAX.into()),
        )?;
        limit(
            "dense atlas height",
            height,
            budget.max_atlas_dimension.min(u32::MAX.into()),
        )?;
        let pixels = width.saturating_mul(height);
        limit("dense atlas pixels", pixels, budget.max_dense_pixels)?;
        // Compaction may temporarily retain dense data, crops, dedup keys,
        // packed output and emission; reserve eight dense RGBA equivalents.
        // Four directions each own a 2x RGBA buffer, 2x f32 z-buffer and low
        // RGBA frame (36 bytes per logical pixel per view).
        let pixel_bytes = pixels.saturating_mul(4).saturating_mul(8);
        let raster_bytes = u64::from(size)
            .saturating_mul(size.into())
            .saturating_mul(36 * views);
        // Conservative scene/pose/metadata allowance, not exact allocator RSS.
        let mesh_bytes = (nodes as u64).saturating_mul(256 * 1024);
        let bytes = pixel_bytes
            .saturating_add(raster_bytes)
            .saturating_add(mesh_bytes)
            .saturating_add(8 * 1024 * 1024);
        let total_bytes = self.reserved_bytes.saturating_add(bytes);
        limit(
            "estimated working bytes",
            total_bytes,
            budget.max_estimated_working_bytes,
        )?;
        // Even explicitly relaxed budgets must not overflow wasm ImageBuffer.
        limit(
            "dense RGBA bytes",
            pixels.saturating_mul(4),
            usize::MAX as u64,
        )?;
        Ok(bytes)
    }

    pub(crate) fn reserve_render(
        &mut self,
        size: u32,
        poses: usize,
        nodes: usize,
    ) -> Result<(), Error> {
        let bytes = self.check_render(size, poses, nodes)?;
        self.reserved_bytes = self.reserved_bytes.saturating_add(bytes);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_limit(result: Result<impl Sized, Error>, resource: &str) {
        assert!(matches!(result, Err(Error::ResourceLimit { resource: r, .. }) if r == resource));
    }

    #[test]
    #[cfg(all(feature = "bert", not(target_arch = "wasm32")))]
    fn native_inference_pool_has_at_most_two_workers() {
        let threads = with_parser_pool(|| Ok(rayon::current_num_threads())).unwrap();
        assert!((1..=2).contains(&threads));
    }

    #[test]
    fn recursive_specs_are_checked_before_anatomy() {
        let state = BudgetState::new(GenerationBudget::default());
        let mut spec = crate::parser::parse_vocabulary("pig riding a pig riding a pig");
        state.check_spec(&spec).unwrap();
        spec.mount.as_mut().unwrap().spec.as_mut().unwrap().heads = 255;
        is_limit(state.check_spec(&spec), "heads per creature");
        let mut spec = crate::parser::parse_vocabulary("pig");
        for _ in 0..4 {
            let mut rider = crate::parser::parse_vocabulary("pig riding a pig");
            rider.mount.as_mut().unwrap().spec = Some(Box::new(spec));
            spec = rider;
        }
        is_limit(state.check_spec(&spec), "mount depth");
    }

    #[test]
    fn layout_limits_and_retained_parent_reservations_are_checked() {
        let mut state = BudgetState::new(GenerationBudget::default());
        is_limit(state.check_render(513, 20, 10), "frame dimension");
        is_limit(state.check_render(96, 513, 10), "animation poses");
        state.limits.max_atlas_dimension = 100;
        is_limit(state.check_render(96, 20, 10), "dense atlas width");
        state.limits = GenerationBudget::default();
        state.limits.max_dense_pixels = 10;
        is_limit(state.check_render(96, 20, 10), "dense atlas pixels");
        state.limits = GenerationBudget::default();
        let one = state.check_render(96, 80, 30).unwrap();
        state.limits.max_estimated_working_bytes = one;
        state.reserve_render(96, 80, 30).unwrap();
        is_limit(state.reserve_render(96, 80, 30), "estimated working bytes");
        // Arithmetic must fail safely even if a caller relaxes every work limit.
        state.limits.max_poses = u64::MAX;
        is_limit(state.check_render(96, usize::MAX, 30), "directional frames");
    }

    #[test]
    fn companions_share_the_package_counter() {
        let mut state = BudgetState::new(GenerationBudget {
            max_packages: 2,
            ..Default::default()
        });
        let mounted = crate::parser::parse_vocabulary("orc riding a wolf");
        is_limit(state.check_spec(&mounted), "package count");
        state.begin_package().unwrap();
        state.begin_package().unwrap();
        is_limit(state.begin_package(), "package count");
    }
}
