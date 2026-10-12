//! Processus auxiliaire autonome : ne charge ni la DLL Flutter ni la fenêtre active.
#[cfg(target_os = "windows")]
fn main() {
    use std::path::Path;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = if args.len() == 6
        && args[0].as_os_str() == std::ffi::OsStr::new("--apply-flutter-update")
    {
        monolith::client_update::apply_flutter_update(
            Path::new(&args[1]),
            Path::new(&args[2]),
            &args[3].to_string_lossy(),
            Path::new(&args[4]),
            Path::new(&args[5]),
        )
    } else {
        Err(anyhow::anyhow!(
            "arguments de mise à jour Flutter invalides"
        ))
    };
    if let Err(error) = result {
        let report = format!("Installation Flutter échouée : {error:#}");
        if let Some(dir) = args.get(5) {
            let _ = std::fs::write(
                Path::new(dir).join("updates/installation-error.txt"),
                &report,
            );
        }
        eprintln!("{report}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("mise à jour Flutter disponible uniquement sur Windows");
}
