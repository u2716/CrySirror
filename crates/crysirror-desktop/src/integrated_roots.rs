//! Bind integrated runtimes to the desktop owner's selected game/save roots.
use anyhow::{Context, Result, ensure};
use crysirror_core::path::{Mount, VfsPath};
use crysirror_runtime::model::Project;
use std::path::Path;
pub(crate) fn bind(
    project: &mut Project,
    assets: &Path,
    state: &Path,
    font: Option<&Path>,
) -> Result<()> {
    project.validate()?;
    let assets = assets.canonicalize().context("integrated asset root")?;
    let fallback = font
        .map(Path::to_path_buf)
        .or_else(super::detect::system_font);
    for client in &mut project.clients {
        if !matches!(client.adapter.as_str(), "bgi" | "softpal") {
            continue;
        }
        let base = assets
            .join(&client.asset_prefix)
            .canonicalize()
            .context("integrated client asset root")?;
        ensure!(
            base.starts_with(&assets) && base.is_dir(),
            "integrated client root escapes assets"
        );
        let config = client
            .config
            .as_object_mut()
            .context("integrated adapter config must be an object")?;
        let relative = if client.adapter == "bgi" {
            config
                .get("root")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .replace('\\', "/")
        } else {
            String::new()
        };
        if !relative.is_empty() {
            VfsPath::new(Mount::Asset, &relative)?;
        }
        let resource = base
            .join(&relative)
            .canonicalize()
            .context("integrated resource root")?;
        ensure!(
            resource.starts_with(&base) && resource.is_dir(),
            "integrated resource root escapes client assets"
        );
        let user = state.join(&client.adapter).join(&client.game).join("save");
        std::fs::create_dir_all(&user).context("integrated native save directory")?;
        let user = user.canonicalize().context("integrated native save root")?;
        config.insert("resource_root".into(), serde_json::to_value(&resource)?);
        config.insert("user_root".into(), serde_json::to_value(user)?);
        if client.adapter == "bgi" {
            config.insert("root".into(), serde_json::json!(""));
        }
        let explicit = font.map(Path::to_path_buf).or_else(|| {
            config
                .get("font_file")
                .and_then(|v| v.as_str())
                .map(Into::into)
        });
        let alias = config
            .get("font")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let declared = project.fonts.get(alias).map(|path| assets.join(path));
        if let Some(path) = explicit.or(declared).or_else(|| fallback.clone()) {
            let path = path.canonicalize().context("integrated runtime font")?;
            ensure!(
                path.is_file(),
                "integrated runtime font is not a regular file"
            );
            config.insert("font_file".into(), serde_json::to_value(path)?);
        }
    }
    Ok(())
}
