use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap();
    
    // Only copy assets in release builds
    if profile == "release" {
        let target_dir = Path::new(&out_dir).parent().unwrap().parent().unwrap().parent().unwrap();
        let assets_src = Path::new("assets");
        let assets_dst = target_dir.join("assets");
        
        if assets_src.exists() {
            // Create assets directory in target
            if !assets_dst.exists() {
                fs::create_dir_all(&assets_dst).unwrap();
            }
            
            // Copy nircmd files
            for entry in fs::read_dir(assets_src).unwrap() {
                let entry = entry.unwrap();
                let src = entry.path();
                let dst = assets_dst.join(entry.file_name());
                
                if src.is_file() {
                    fs::copy(&src, &dst).unwrap();
                    println!("cargo:rerun-if-changed={}", src.display());
                }
            }
            
            // Also copy nircmd.exe to the main target directory for easier access
            let nircmd_src = assets_src.join("nircmd.exe");
            let nircmd_dst = target_dir.join("nircmd.exe");
            if nircmd_src.exists() {
                fs::copy(&nircmd_src, &nircmd_dst).unwrap();
            }
        }
    }
    
    println!("cargo:rerun-if-changed=assets");
}