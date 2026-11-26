// SPDX-License-Identifier: MPL-2.0
use super::*;
#[derive(Default)]
pub(super) struct Show {
    entrants: Vec<usize>,
    since: Option<u64>,
    simplified: bool,
    success: bool,
    sound: bool,
    results: Vec<(usize, Vec<String>)>,
    page: usize,
    motion: Option<(String, String)>,
}
impl Game {
    pub(super) fn open_examination(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        now: u64,
    ) -> Result<String> {
        self.main_light.rotation = [28.99218, 178.2424, 16.81662];
        self.main_light.intensity = 0.95;
        self.adv_camera = ([0.08306749, 1.299064, 0.0249091], 2.2, [177.9044, 2.546583]);
        {
            let vm = self
                .script
                .tjs
                .lock()
                .map_err(|_| anyhow::anyhow!("TJS lock"))?;
            self.leisure.examination.entrants =
                activities::show_candidates(&vm.runtime.host().state);
        }
        ensure!(
            !self.leisure.examination.entrants.is_empty(),
            "no exhibition entrants from character selection"
        );
        if self.leisure.examination.entrants.len() > 1
            && !self
                .script
                .tjs
                .lock()
                .map_err(|_| anyhow::anyhow!("TJS lock"))?
                .runtime
                .host()
                .state
                .retail
                .show_in_progress
        {
            self.leisure_prompt(
                op,
                host,
                c,
                now,
                "演出を簡略化しますか?",
                "yesno",
                Response::Simplify,
            )?;
        } else {
            self.start_judging(false, op, host, c, now)?;
        }
        Ok("Theater".into())
    }
    pub(super) fn start_judging(
        &mut self,
        all: bool,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        now: u64,
    ) -> Result<()> {
        let show = &mut self.leisure.examination;
        show.simplified = all;
        let mut id = show.entrants[0];
        if !all {
            show.entrants.truncate(1);
        }
        {
            let mut vm = self
                .script
                .tjs
                .lock()
                .map_err(|_| anyhow::anyhow!("TJS lock"))?;
            let s = &mut vm.runtime.host_mut().state;
            activities::simplify_show(s, all);
            for &candidate in &show.entrants {
                if !crate::tjs::retail::examine(s, candidate, false)?.is_empty() {
                    show.success = true;
                    id = candidate;
                    break;
                }
            }
            let list = s.retail.table("maid_status_personal_list.nei")?;
            let r = (1..list.rows)
                .find(|&r| list.int(0, r) == s.maids[&id].personal)
                .context("examination personality")?;
            let entry = list.string(if show.success { 6 } else { 7 }, r);
            let (file, label) = entry
                .split_once('/')
                .context("missing examination motion label")?;
            show.motion = Some((file.into(), label.into()));
        }
        show.since = Some(now);
        self.leisure_actor(id, op, now)?;
        let head = self.actors.examination_head(&self.files, op, now)?;
        self.adv_camera = (
            [0.03583635, 0.3097733, -0.4142513],
            0.7,
            [-18.80226, 11.24228],
        );
        self.camera_move = Some(AdvCameraMove {
            from: self.adv_camera,
            to: (
                [
                    head[0] - 0.034305904,
                    head[1] - 0.3253325,
                    head[2] - 0.48361522,
                ],
                2.8,
                [-182.2753, 11.24231],
            ),
            start: now + 500,
            duration: 5500,
            ease: "easeOutCubic".into(),
        });
        self.sound
            .play(host, c, &self.files, Kind::Se, "SE005.ogg", 0., false)?;
        Ok(())
    }
    pub(super) fn tick_examination(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        now: u64,
    ) -> Result<()> {
        let show = &mut self.leisure.examination;
        let Some(start) = show.since else {
            return Ok(());
        };
        let elapsed = now.saturating_sub(start);
        if elapsed >= 6500 && !show.sound {
            show.sound = true;
            if show.success {
                self.sound
                    .play(host, c, &self.files, Kind::Se, "SE007.ogg", 0., false)?;
                self.sound
                    .play(host, c, &self.files, Kind::Se, "SE008.ogg", 0., false)?;
                self.sound
                    .play(host, c, &self.files, Kind::Bgm, "BGM007.ogg", 0., false)?;
            } else {
                self.sound
                    .play(host, c, &self.files, Kind::Se, "SE021.ogg", 0., false)?;
            }
        }
        if elapsed >= 6500
            && let Some((file, label)) = show.motion.take()
        {
            let success = show.success;
            self.leisure_motion(&file, &label, op, now)?;
            self.actors.apply(
                crate::kag::AdvUi::Face {
                    man: false,
                    slot: 0,
                    name: if success { "にっこり" } else { "泣き" }.into(),
                },
                &self.script,
                &self.files,
                &mut self.assets,
                op,
                now,
            )?;
        }
        let show = &mut self.leisure.examination;
        if elapsed < 9500 {
            return Ok(());
        }
        show.since = None;
        {
            let mut vm = self
                .script
                .tjs
                .lock()
                .map_err(|_| anyhow::anyhow!("TJS lock"))?;
            let s = &mut vm.runtime.host_mut().state;
            for &id in &show.entrants {
                let gained = crate::tjs::retail::examine(s, id, true)?;
                if !gained.is_empty() {
                    show.results.push((id, gained));
                }
            }
        }
        if show.results.is_empty() {
            let message = if show.simplified {
                "今回は新しいクラスを\n誰も貰えませんでした。"
            } else {
                "新しいクラスを貰えませんでした。"
            };
            return self.leisure_prompt(op, host, c, now, message, "ok", Response::EndShow);
        }
        self.show_result()
    }
    fn show_result(&mut self) -> Result<()> {
        let show = &self.leisure.examination;
        let (id, class) = &show.results[show.page];
        let vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let m = &vm.runtime.host().state.maids[id];
        let last = show.page + 1 == show.results.len();
        self.widgets = self
            .leisure
            .base
            .iter()
            .filter(|w| !(w.path.ends_with("/Ok") && !last || w.path.ends_with("/Next") && last))
            .cloned()
            .collect();
        for w in &mut self.widgets {
            if let Some(l) = &mut w.label {
                if w.path.ends_with("/FirstName") {
                    l.set_text(&m.first_name);
                } else if w.path.ends_with("/LastName") {
                    l.set_text(&m.last_name);
                } else if w.path.ends_with("/MessageBox") {
                    l.set_text(&class.join("、"));
                }
            }
        }
        Ok(())
    }
    pub(super) fn examination_click(
        &mut self,
        path: &str,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        now: u64,
    ) -> Result<()> {
        if path.ends_with("/Next") {
            self.leisure.examination.page += 1;
            self.show_result()?;
        }
        if path.ends_with("/Ok") {
            if self.leisure.examination.simplified {
                self.widgets.clear();
                return self.leisure_prompt(
                    op,
                    host,
                    c,
                    now,
                    "今回新しくクラスを\n獲得したメイドは以上です。",
                    "ok",
                    Response::EndShow,
                );
            }
            self.resume_script(op, host, c, now)?;
        }
        Ok(())
    }
}
