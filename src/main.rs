use std::path::Path;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if (args.len() == 2 || args.len() == 3) && args[1] == "serve" {
        let port = args.get(2).map_or(Ok(8799), |value| value.parse::<u16>());
        let port = port.unwrap_or_else(|_| {
            eprintln!("port must be 0–65535");
            std::process::exit(2)
        });
        if let Err(error) = infernal_diffusion::web::serve(port, Path::new("web/generated")) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if (args.len() == 3 || args.len() == 4) && args[1] == "parse" {
        let spec = if let Some(model_dir) = args.get(3) {
            #[cfg(feature = "bert")]
            {
                infernal_diffusion::parser::parse_with_bert(&args[2], Path::new(model_dir))
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "bert"))]
            {
                Err(format!("rebuild with --features bert to load {model_dir}"))
            }
        } else {
            #[cfg(feature = "bert")]
            {
                infernal_diffusion::parser::parse_with_embedded_bert(&args[2])
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "bert"))]
            {
                Ok(infernal_diffusion::parser::parse_vocabulary(&args[2]))
            }
        };
        match spec.and_then(|value| serde_json::to_string_pretty(&value).map_err(|e| e.to_string()))
        {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1)
            }
        }
        return;
    }
    if args.len() == 3 && (args[1] == "validate" || args[1] == "inspect") {
        match infernal_diffusion::load_package(Path::new(&args[2])) {
            Ok(monster) if args[1] == "inspect" => {
                println!("{}: {}", monster.id, monster.display_name);
                println!("{}", monster.description);
                if let Some(generation) = &monster.generation {
                    println!("prompt: {}", generation.prompt);
                    println!("seed: {}", generation.seed);
                }
                println!("animations: {}", monster.animations.len());
                println!("movement modes: {}", monster.movement_modes.len());
                println!("attacks: {}", monster.attacks.len());
                if let Some(size) = &monster.size {
                    println!(
                        "size: {} ({:.2}), visible {}x{} px",
                        size.size_class,
                        size.normalized_size,
                        size.visible_width_px,
                        size.visible_height_px
                    );
                }
                if let Some(gameplay) = &monster.gameplay {
                    println!(
                        "health: {} (base 20 + size {} + armor {} + magic {})",
                        gameplay.health,
                        gameplay.health_size_bonus,
                        gameplay.health_armor_bonus,
                        gameplay.health_magic_bonus
                    );
                }
            }
            Ok(monster) => println!("valid: {}", monster.id),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1)
            }
        }
        return;
    }
    if (args.len() == 5 || args.len() == 6) && (args[1] == "bake3d" || args[1] == "bake2d") {
        let seed: u64 = args[3].parse().unwrap_or_else(|_| {
            eprintln!("seed must be an unsigned integer");
            std::process::exit(2)
        });
        let bake = if args[1] == "bake2d" {
            infernal_diffusion::generate_2d
        } else {
            infernal_diffusion::generate_3d
        };
        match bake(
            &args[2],
            seed,
            Path::new(&args[4]),
            args.get(5).map(Path::new),
        ) {
            Ok(monster) => println!("{}: {}", monster.id, monster.display_name),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1)
            }
        }
        return;
    }
    if args.len() < 4 || args.len() > 5 {
        eprintln!("Usage: infernal <prompt> <seed> <output-dir> [bert-model-dir]");
        eprintln!("       infernal validate <package-dir>");
        eprintln!("       infernal inspect <package-dir>");
        eprintln!("       infernal parse <prompt> [bert-model-dir]");
        eprintln!("       infernal bake3d <prompt> <seed> <output-dir> [bert-model-dir]");
        eprintln!("       infernal bake2d <prompt> <seed> <output-dir> [bert-model-dir]");
        eprintln!("       infernal serve [port]");
        std::process::exit(2);
    }
    let seed: u64 = match args[2].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("seed must be an unsigned integer");
            std::process::exit(2)
        }
    };
    match infernal_diffusion::generate(
        &args[1],
        seed,
        Path::new(&args[3]),
        args.get(4).map(Path::new),
    ) {
        Ok(m) => println!("{}: {}", m.id, m.display_name),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
    }
}
