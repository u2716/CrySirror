// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native sound directory/extension precedence over shared capability lookup.
use super::*;
impl SiglusVm {
    /// Native voice number: pack `number / 100000`, sample `number % 100000`,
    /// in a mounted `koe/z<pack>.ovk|nwk` or as a loose `koe/<pack>/z<number>` file.
    pub(super) fn koe_asset(&self, host: &mut dyn HostServices, number: i64) -> Result<String> {
        if let Some(alias) = self.state.config.resources.get(&format!("koe/{number}")) {
            return Ok(alias.clone());
        }
        ensure!(number >= 0, "negative Siglus voice number {number}");
        let (pack, sample) = (number / 100_000, number % 100_000);
        let storage = host.storage(ClientId(self.state.client))?;
        for source in [
            format!("koe/z{pack:04}.ovk"), format!("koe/Z{pack:04}.ovk"),
            format!("koe/z{pack:04}.nwk"), format!("koe/Z{pack:04}.nwk"),
        ] {
            let Some(point) = storage.container_mount(&source)? else { continue };
            let stem = format!("sample{sample:05}");
            let names = storage.list(Mount::Asset, &point)?;
            if let Some(name) = names.iter().find(|n| n.split('.').next() == Some(stem.as_str())) {
                return Ok(format!("{point}/{name}"));
            }
        }
        for extension in ["nwa", "ogg", "wav", "owp"] {
            let loose = format!("koe/{pack:04}/z{pack:04}{sample:05}.{extension}");
            if let Some(path) = crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                storage.as_ref(),
                &VfsPath::new(Mount::Asset, &loose)?,
            )? {
                return Ok(path.relative().to_owned());
            }
        }
        anyhow::bail!("Siglus voice {number} is not in pack {pack} or its loose folder")
    }
    pub(super) fn audio_asset(
        &self,
        host: &mut dyn HostServices,
        name: &str,
        directory: &str,
    ) -> Result<String> {
        if let Some(alias) = self.state.config.resources.get(name) {
            return Ok(alias.clone());
        }
        if let Some(number) = name.strip_prefix("koe/").and_then(|n| n.parse::<i64>().ok()) {
            return self.koe_asset(host, number);
        }
        ensure!(
            self.state
                .vm
                .as_ref()
                .unwrap()
                .ctx
                .globals
                .append_dir
                .is_empty(),
            "native sound append search needs an explicit resource alias"
        );
        let name = name.replace('\\', "/");
        VfsPath::new(Mount::Asset, &name)?;
        ensure!(
            matches!(directory, "wav" | "bgm"),
            "unsupported native sound resource directory"
        );
        let storage = host.storage(ClientId(self.state.client))?;
        let find = |candidate: &str| -> Result<Option<String>> {
            Ok(
                crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                    storage.as_ref(),
                    &VfsPath::new(Mount::Asset, candidate)?,
                )?
                .map(|p| p.relative().to_owned()),
            )
        };
        if name.contains('/') {
            if let Some(path) = find(&name)? {
                return Ok(path);
            }
        }
        let extension = std::path::Path::new(&name).extension();
        if let Some(extension) = extension {
            ensure!(
                matches!(
                    extension
                        .to_str()
                        .unwrap_or("")
                        .to_ascii_lowercase()
                        .as_str(),
                    "wav" | "nwa" | "ogg" | "owp" | "ovk"
                ),
                "unsupported native sound extension"
            );
            if let Some(path) = find(&format!("{directory}/{name}"))? {
                return Ok(path);
            }
        } else {
            // Fixed pinned Siglus WAV_ORDER; never a codec-availability fallback.
            for extension in ["wav", "nwa", "ogg", "owp", "ovk"] {
                if let Some(path) = find(&format!("{directory}/{name}.{extension}"))? {
                    return Ok(path);
                }
            }
        }
        anyhow::bail!("native sound resource not found: {name}")
    }
}
