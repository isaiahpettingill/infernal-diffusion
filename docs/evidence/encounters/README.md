# First encounter batch: actual rendered evidence

These images use the `profile.v2@4` recipe identity and come from the CPU sprite baker. They are not concept art or manually redrawn silhouettes.

- `encounters-four-directions.png`: seed 7, all four baked directions; mimics use their closed `v2_lure` clip
- `encounters-three-seeds.png`: seeds 0, 7 and 42, side and front views
- `encounters-keyframes.png`: chronological hop, emergence, burst, reveal, bash/recovery and push/recovery frames
- `*-motion.gif`: each identity's action clips in all four directions
- `v1-compatibility.json`: eleven terrestrial V1 specimens remain byte-identical to the pinned pre-V2 baseline on the same host
- `render-audit.json`: every directional frame in all 18 packages checked for alpha touching its frame edge

The bush, banana and amphora stay environmental objects in the lure clip. The reveal opens their real jaw surfaces and exposes teeth. Normal idle/movement clips stay revealed. The sentinel lowers its shield into recovery to expose its chest core. The hunched pusher has two empty hands; the independently simulated boulder is intentionally absent from every actor sprite.

Reproduce from the repository root:

```sh
cargo run --release --no-default-features --example encounter_catalog -- output/encounters
python3 tools/encounter_evidence.py output/encounters docs/evidence/encounters
```

The catalog helper also accepts an optional comma-separated identity filter and seed list. For example, `output/encounters banana 7` regenerates the banana at seed 7.
