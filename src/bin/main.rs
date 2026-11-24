use anyhow::Result;
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args]", args[0]);
        eprintln!("Commands:");
        eprintln!("  ls <path>          - List directory");
        eprintln!("  cat <path>         - Read file");
        eprintln!("  apk <path>         - Parse APK");
        eprintln!("  dex <path>         - Parse DEX");
        return Ok(());
    }

    let cmd = &args[1];

    match cmd.as_str() {
        "ls" => {
            if args.len() < 3 {
                eprintln!("Usage: {} ls <path>", args[0]);
                return Ok(());
            }
            let path = Path::new(&args[2]);
            match mtman::fs::list_dir(path) {
                Ok(entries) => {
                    for entry in entries {
                        let ft = if entry.is_dir { "d" } else { "-" };
                        let size = if entry.is_dir { "-".to_string() } else { entry.size.to_string() };
                        println!("{} {:>10} {}", ft, size, entry.name);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        "cat" => {
            if args.len() < 3 {
                eprintln!("Usage: {} cat <path>", args[0]);
                return Ok(());
            }
            let path = Path::new(&args[2]);
            match mtman::fs::read_file(path) {
                Ok(data) => {
                    println!("{}", String::from_utf8_lossy(&data));
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        "apk" => {
            if args.len() < 3 {
                eprintln!("Usage: {} apk <path>", args[0]);
                return Ok(());
            }
            let path = Path::new(&args[2]);
            match mtman::apk::Apk::open(path) {
                Ok(apk) => {
                    println!("APK: {}", apk.path.display());
                    println!("Entries: {}", apk.entries.len());
                    for (name, entry) in &apk.entries {
                        println!("  {} ({}) - {} bytes", name, entry.method, entry.size);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        "dex" => {
            if args.len() < 3 {
                eprintln!("Usage: {} dex <path>", args[0]);
                return Ok(());
            }
            let path = Path::new(&args[2]);
            match mtman::fs::read_file(path) {
                Ok(data) => match mtman::dex::Dex::parse(&data) {
                    Ok(dex) => {
                        println!("DEX: {:?}", dex.header.magic);
                        println!("Strings: {}", dex.header.string_ids_size);
                        println!("Types: {}", dex.header.type_ids_size);
                        println!("Methods: {}", dex.header.method_ids_size);
                        println!("Classes: {}", dex.header.class_defs_size);
                    }
                    Err(e) => eprintln!("Error: {}", e),
                },
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        "apkcat" => {
            if args.len() < 4 {
                eprintln!("Usage: {} apkcat <apk> <entry>", args[0]);
                return Ok(());
            }
            let apk = mtman::apk::Apk::open(Path::new(&args[2]))?;
            match apk.read(&args[3]) {
                Ok(data) => {
                    println!("{} bytes", data.len());
                    if data.len() < 4096 {
                        println!("{}", String::from_utf8_lossy(&data));
                    }
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "arsc" => {
            if args.len() < 3 {
                eprintln!("Usage: {} arsc <resources.arsc>", args[0]);
                return Ok(());
            }
            let data = mtman::fs::read_file(Path::new(&args[2]))?;
            let arsc = mtman::arsc::Arsc::parse(&data)?;
            println!("global strings: {}", arsc.global_strings.len());
            for p in &arsc.packages {
                println!("package {} ({}) types={}", p.name, p.id, p.types.len());
                for t in p.types.iter().take(20) {
                    println!("  type {} {} entries={}", t.id, t.name, t.entry_count);
                }
            }
        }
        "dexclasses" => {
            if args.len() < 3 {
                eprintln!("Usage: {} dexclasses <classes.dex>", args[0]);
                return Ok(());
            }
            let data = mtman::fs::read_file(Path::new(&args[2]))?;
            let dex = mtman::dex::Dex::parse(&data)?;
            let names = dex.class_names()?;
            println!("classes: {}", names.len());
            for n in names.iter().take(30) {
                println!("  {n}");
            }
        }
        "dexstrings" => {
            if args.len() < 4 {
                eprintln!("Usage: {} dexstrings <classes.dex> <needle>", args[0]);
                return Ok(());
            }
            let data = mtman::fs::read_file(Path::new(&args[2]))?;
            let dex = mtman::dex::Dex::parse(&data)?;
            let hits = dex.find_strings(&args[3])?;
            println!("{} hits", hits.len());
            for (i, s) in hits.iter().take(30) {
                println!("  [{i}] {s}");
            }
        }
        _ => {
            eprintln!("Unknown command: {}", cmd);
        }
    }

    Ok(())
}
