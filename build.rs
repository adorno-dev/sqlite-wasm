use std::fs;
use std::path::Path;
use std::env;
#[allow(unused)]
use std::collections::HashMap;

// fn main() {
//     // Mantém a funcionalidade original de copiar (para desenvolvimento local)
//     let src_dir = Path::new("jswasm");
//     let dest_dir = Path::new("pkg/jswasm");
//
//     if !dest_dir.exists() {
//         fs::create_dir_all(dest_dir).unwrap();
//     }
//
//     for entry in fs::read_dir(src_dir).unwrap() {
//         let entry = entry.unwrap();
//         let src_path = entry.path();
//         let dest_path = dest_dir.join(entry.file_name());
//         fs::copy(src_path, dest_path).unwrap();
//     }
//
//     println!("cargo:rerun-if-changed=jswasm/");
// }

fn main() {
    let src_dir = Path::new("jswasm");
    let out_dir = env::var("OUT_DIR").unwrap();
    
    // ===================================================
    // 1. SEMPRE: Copia jswasm para OUT_DIR
    // ===================================================
    let dest_out = Path::new(&out_dir).join("jswasm");
    if !dest_out.exists() {
        fs::create_dir_all(&dest_out).unwrap();
    }
    
    for entry in fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dest_path = dest_out.join(entry.file_name());
        fs::copy(&src_path, &dest_path).unwrap();
        println!("cargo:warning=📋 Copiado para OUT_DIR: {:?}", entry.file_name());
    }
    
    // ===================================================
    // 2. Se for uma DEPENDÊNCIA (importado por outro projeto)
    // ===================================================
    if env::var("CARGO_PRIMARY_PACKAGE").is_err() {
        println!("cargo:warning=📦 Detectado: sendo usado como dependência!");
        
        // Encontra a raiz do projeto que está importando
        if let Ok(current_dir) = env::current_dir() {
            let mut project_root = current_dir.clone();
            
            // Sobe até encontrar Cargo.toml (raiz do projeto)
            while !project_root.join("Cargo.toml").exists() {
                if !project_root.pop() {
                    break;
                }
            }
            
            if project_root.join("Cargo.toml").exists() {
                // Copia de OUT_DIR para a raiz do projeto que importa
                let dest_project = project_root.join("jswasm");  // /projeto-dele/jswasm
                fs::create_dir_all(&dest_project).unwrap();
                
                for entry in fs::read_dir(&dest_out).unwrap() {
                    let entry = entry.unwrap();
                    let src_path = entry.path();
                    let dest_path = dest_project.join(entry.file_name());
                    fs::copy(&src_path, &dest_path).unwrap();
                    println!("cargo:warning=📋 Copiado para RAIZ: {:?}", entry.file_name());
                }
                
                println!("cargo:warning=✅ jswasm copiado para: {}/jswasm", project_root.display());
            }
        }
    }
    
    // ===================================================
    // 3. Também copia para pkg/ (desenvolvimento local da crate)
    // ===================================================
    let dest_pkg = Path::new("pkg").join("jswasm");
    if !dest_pkg.exists() {
        fs::create_dir_all(&dest_pkg).unwrap();
    }
    
    for entry in fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dest_path = dest_pkg.join(entry.file_name());
        fs::copy(&src_path, &dest_path).unwrap();
    }
    
    println!("cargo:rerun-if-changed=jswasm/");
}
