use sagewatch_lib::{
    domain::Provider,
    service::{AppSnapshot, RefreshService},
};
use std::{env, path::PathBuf};

fn app_data_dir() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("com.sagewatch.desktop"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/share/com.sagewatch.desktop"))
        .ok_or_else(|| "HOME or XDG_DATA_HOME is required".to_owned())
}

async fn snapshot(refresh: bool) -> Result<AppSnapshot, String> {
    let service = RefreshService::bootstrap(app_data_dir()?).map_err(|error| error.to_string())?;
    if refresh {
        let _ = tokio::join!(
            service.refresh_provider(Provider::Claude),
            service.refresh_provider(Provider::Codex)
        );
    }
    Ok(service.snapshot().await)
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    let refresh_entry_point = arguments
        .first()
        .is_some_and(|path| path.ends_with("sagewatch-plasma-refresh"));
    let refresh = match arguments.get(1).map(String::as_str) {
        None | Some("status") => false,
        Some("refresh") => true,
        Some(_) => {
            eprintln!("usage: sagewatch-plasma-provider [status|refresh]");
            std::process::exit(2);
        }
    } || refresh_entry_point;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create async runtime");
    match runtime
        .block_on(snapshot(refresh))
        .and_then(|value| serde_json::to_string(&value).map_err(|error| error.to_string()))
    {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::app_data_dir;
    #[test]
    fn data_path_uses_the_desktop_application_identifier() {
        assert!(app_data_dir().unwrap().ends_with("com.sagewatch.desktop"));
    }
}
