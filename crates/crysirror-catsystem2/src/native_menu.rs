// SPDX-License-Identifier: MPL-2.0
//! Native resource IDs/caption policy translated onto shared retained menus.
use anyhow::{ensure, Context, Result};
use crysirror_resource::{
    pe::PeResourceName,
    pe_menu::{MenuTemplate, MenuTemplateItem},
};
use crysirror_scene::ui::menu::{MenuItemSpec, MenuSpec};
use std::collections::{BTreeSet, VecDeque};
/// Production menu resource in the statically reviewed native ABI.
pub const PRODUCTION_MENU: u32 = 104;
/// Native command disabled by GMenuReturnTitle during system initialization.
pub const RETURN_TITLE: u32 = 119;
pub(crate) const SYSTEM_INFORMATION: u32 = 40019;
pub(crate) const ABOUT: u32 = 40041;

/// 530230/52F610: native information commands open synchronous, dismissible
/// dialogs. The host owns their retained UI; the adapter pauses its VM until
/// the dialog closes. Host metadata preparation stays off the owner thread.
#[derive(Default)]
pub(crate) struct State {
    queued: VecDeque<u32>,
    preparing: Option<(u32, crysirror_tasks::Task<Option<usize>>)>,
    active: Option<u32>,
}
impl State {
    pub(crate) fn modal_active(&self) -> bool { self.active.is_some() }
    pub(crate) fn enqueue(&mut self, command: u32) -> Result<()> {
        ensure!(matches!(command, SYSTEM_INFORMATION | ABOUT), "unsupported CatSystem2 native menu WM_COMMAND {command}");
        ensure!(self.queued.len() < 16, "CatSystem2 menu command queue budget");
        self.queued.push_back(command);
        Ok(())
    }
    /// True while the modal owner must retain VM/input execution.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn tick(&mut self, host: &mut dyn crysirror_core::service::HostServices, client: crysirror_core::state::ClientId, pool: &crysirror_tasks::TaskPool, canvas: [u32; 2], font: &str, module: Option<&str>) -> Result<bool> {
        use crysirror_core::service::EngineQuery;
        use crysirror_scene::ui::dialog::{DialogButton, DialogSpec};
        if let Some(id) = self.active {
            let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_slice(&host.query_engine(client, EngineQuery::DialogEvents)?)?;
            for event in events {
                ensure!(event.kind == "dialog_result" && event.widget.id == id && event.widget.owner == client.0, "CatSystem2 unexpected native menu dialog completion");
                ensure!(event.value == 1. || event.value == 2., "CatSystem2 native menu dialog button");
                self.active = None;
            }
            // Do not offer the Enter/Escape edge which dismissed the modal
            // dialog to the game in the same owner tick.
            return Ok(true);
        }
        if self.preparing.is_none() {
            let Some(command) = self.queued.front().copied() else { return Ok(false) };
            let permit = match pool.reserve() {
                Ok(permit) => permit,
                Err(error) if error.is::<crysirror_tasks::CapacityExceeded>() => return Ok(true),
                Err(error) => return Err(error),
            };
            let task = permit.spawn(move |cancel| {
                cancel.check()?;
                let threads = std::thread::available_parallelism().ok().map(usize::from);
                cancel.check()?;
                Ok(threads)
            })?;
            self.queued.pop_front();
            self.preparing = Some((command, task));
            return Ok(true);
        }
        let (command, task) = self.preparing.as_mut().unwrap();
        let Some(threads) = task.try_take()? else { return Ok(true) };
        let command = *command;
        let (title, message) = if command == SYSTEM_INFORMATION {
            let mut rows = vec![
                format!("実行エンジン: CrySirror {}", env!("CARGO_PKG_VERSION")),
                "互換VM: CatSystem2 / KCS".to_owned(),
                format!("OS: {}", std::env::consts::OS),
                format!("プロセスアーキテクチャ: {}", std::env::consts::ARCH),
                format!("利用可能な論理CPU数: {}", threads.map_or_else(|| "利用不可".to_owned(), |count| count.to_string())),
                format!("ゲーム画面: {} × {}", canvas[0], canvas[1]),
            ];
            if let Some(display) = host.display_info() { rows.push(format!("ディスプレイ: {} × {}", display.screen[0], display.screen[1])); }
            if let Some(window) = host.window_geometry(client)? { rows.push(format!("ウィンドウ内容: {} × {}", window.inner_size[0], window.inner_size[1])); }
            if let Some(module) = module { rows.push(format!("ゲームモジュール: {module}")); }
            ("システム情報", rows.join("\n"))
        } else {
            ("バージョン情報", format!("CrySirror {}\nCatSystem2 KCS compatibility VM\n{}", env!("CARGO_PKG_VERSION"), module.unwrap_or("")))
        };
        let dialog = DialogSpec { id: command, title: title.to_owned(), message, buttons: vec![DialogButton { id: 1, label: "OK".to_owned() }], default: 1, cancel: Some(1), font: font.to_owned(), text_size: 22 };
        dialog.validate()?;
        host.submit_engine(client, 1, &serde_json::to_vec(&serde_json::json!({"ui":[{"op":"open_dialog","dialog":dialog}]}))?)?;
        self.preparing = None;
        self.active = Some(command);
        Ok(true)
    }
}
pub fn build(template: &MenuTemplate, canvas: [u32; 2], font: &str) -> Result<MenuSpec> {
    let mut used = BTreeSet::new();
    fn collect(items: &[MenuTemplateItem], used: &mut BTreeSet<u32>) -> Result<()> {
        for i in items {
            if let Some(id) = i.id.filter(|id| *id != 0) {
                ensure!(
                    (1..=65535).contains(&id) && used.insert(id),
                    "unsupported duplicate/out-of-range native menu command"
                );
            }
            collect(&i.children, used)?;
        }
        Ok(())
    }
    collect(&template.items, &mut used)?;
    ensure!(
        used.contains(&RETURN_TITLE),
        "native production menu has no Return to title command"
    );
    let mut next = 65535;
    fn convert(
        items: &[MenuTemplateItem],
        used: &mut BTreeSet<u32>,
        next: &mut u32,
    ) -> Result<Vec<MenuItemSpec>> {
        items
            .iter()
            .map(|item| {
                ensure!(
                    item.item_type & !0x800 == 0,
                    "unsupported native menu type flags {:#x}",
                    item.item_type
                );
                let id = if let Some(id) = item.id.filter(|id| *id != 0) {
                    id
                } else {
                    while used.contains(next) {
                        *next = next
                            .checked_sub(1)
                            .context("native menu logical ID exhaustion")?;
                    }
                    let id = *next;
                    used.insert(id);
                    *next = next
                        .checked_sub(1)
                        .context("native menu logical ID exhaustion")?;
                    id
                };
                let (label, shortcut) = item.text.split_once('\t').unwrap_or((&item.text, ""));
                Ok(MenuItemSpec {
                    id,
                    label: strip_language_tag(label).into(),
                    visible: true,
                    enabled: item.state & 3 == 0,
                    checked: item.state & 8 != 0,
                    radio: false,
                    group: 0,
                    separator: item.item_type & 0x800 != 0
                        || (item.id == Some(0) && item.text.is_empty()),
                    shortcut: shortcut.into(),
                    children: convert(&item.children, used, next)?,
                })
            })
            .collect()
    }
    let menu = MenuSpec {
        id: PRODUCTION_MENU,
        rect: [0., 0., canvas[0] as f32, 24.],
        font: font.into(),
        text_size: 18,
        cell_width: 120.,
        popup_width: 360.,
        row_height: 24.,
        visible: true,
        context: false,
        items: convert(&template.items, &mut used, &mut next)?,
    };
    menu.validate()?;
    Ok(menu)
}
/// RT_MENU captions carry a language table key (`$0011-0001$システム(S)`) ahead of
/// the default text; the key is never shown to a player.
fn strip_language_tag(label: &str) -> &str {
    let Some(rest) = label.strip_prefix('$') else { return label };
    match rest.split_once('$') {
        Some((key, text)) if !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit() || b == b'-') => text,
        _ => label,
    }
}
pub fn production(resources: &[crysirror_resource::pe::PeMenuResource]) -> Result<&MenuTemplate> {
    let mut menus = resources
        .iter()
        .filter(|m| m.name == PeResourceName::Id(PRODUCTION_MENU) && m.language == 1041);
    let first = menus
        .next()
        .context("reviewed native RT_MENU #104 language 1041 missing")?;
    ensure!(menus.next().is_none(), "duplicate native production menu");
    Ok(&first.template)
}
