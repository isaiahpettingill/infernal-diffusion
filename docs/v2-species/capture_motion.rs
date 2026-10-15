use infernal_diffusion::{generate_with_profile,GenerationProfile,PackageFormat};
use std::path::Path;
fn main() {
 let args:Vec<_>=std::env::args().collect();
 let path=Path::new(&args[1]); let prompt=&args[2]; let seed=args[3].parse().unwrap();
 match generate_with_profile(prompt,seed,path,None,PackageFormat::Cbor,GenerationProfile::V2) {
  Ok(monster)=>{ std::fs::write(path.join("metadata.json"),serde_json::to_vec_pretty(&monster).unwrap()).unwrap(); println!("OK {} {}",path.display(),monster.id); },
  Err(error)=>{eprintln!("FAILED {}: {}",prompt,error); std::process::exit(1);}
 }
}
