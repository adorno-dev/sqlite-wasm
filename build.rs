use std::fs;
use std::path::Path;

fn main() {
    let src_dir = Path::new("jswasm");
    let dest_dir = Path::new("pkg/jswasm");

    // Cria o diretório de destino se não existir
    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir).unwrap();
    }

    // Copia todos os arquivos da pasta jswasm para a pasta pkg/jswasm
    for entry in fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dest_path = dest_dir.join(entry.file_name());

        // Copia os arquivos individuais
        fs::copy(src_path, dest_path).unwrap();
    }

    println!("cargo:rerun-if-changed=jswasm/");
}
