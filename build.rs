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
    // 1. SEMPRE: Copia jswasm para OUT_DIR (é isso que você quer!)
    // ===================================================
    let dest_out = Path::new(&out_dir).join("jswasm");
    fs::create_dir_all(&dest_out).unwrap();
    
    for entry in fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dest_out.join(entry.file_name())).unwrap();
        println!("cargo:warning=📋 Copiado para OUT_DIR: {:?}", entry.file_name());
    }
    
    println!("cargo:warning=✅ jswasm copiado para OUT_DIR: {:?}", dest_out);
    
    // ===================================================
    // 2. SÓ COPIA PARA A RAIZ se for OUTRO PROJETO usando
    // ===================================================
    // Só faz isso se NÃO for o projeto principal E se o diretório for diferente
    if env::var("CARGO_PRIMARY_PACKAGE").is_err() {
        // Verifica se está em um diretório diferente do projeto da crate
        if let Ok(current_dir) = env::current_dir() {
            let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
            
            if current_dir != crate_dir {
                println!("cargo:warning=📦 Detectado: projeto diferente!");
                
                // Aqui copia para a raiz do projeto que está importando
                let dest_project = current_dir.join("jswasm");
                fs::create_dir_all(&dest_project).unwrap();
                
                for entry in fs::read_dir(&dest_out).unwrap() {
                    let entry = entry.unwrap();
                    fs::copy(entry.path(), dest_project.join(entry.file_name())).unwrap();
                    println!("cargo:warning=📋 Copiado para raiz: {:?}", entry.file_name());
                }
            }
        }
    }
    
    // ===================================================
    // 3. Copia para pkg/ (desenvolvimento local)
    // ===================================================
    let dest_pkg = Path::new("pkg").join("jswasm");
    fs::create_dir_all(&dest_pkg).unwrap();
    
    for entry in fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dest_pkg.join(entry.file_name())).unwrap();
    }
    
    println!("cargo:rerun-if-changed=jswasm/");
}
