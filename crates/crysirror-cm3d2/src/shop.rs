// SPDX-License-Identifier: MPL-2.0
//! `SceneShop`: `GameInShop` (`Main`, `Move`) and `GameInShopMain` (main and
//! sub category tabs over the player's shop lineup, `ShopItem` units with
//! purchase dialogs, group items, set cards, trial wear on the leader maid).
use crate::{
    adapter::{Game, Opener},
    gamedata::{GameData, ShopItem},
    kag::{AdvUi, EyeLook},
    params::{PlayerStatus, SHOP_NEW, SHOP_PURCHASED},
    sound::{Kind, SoundOp},
    wf::{self, Child, FadeStatus, Manager, Screens},
};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

const ROOT: &str = "UI Root/Parent";
const MAIN_CATS: &str = "UI Root/Parent/MainCategoryViewer/Content/CategoryUnitParent";
const SUB_CATS: &str = "UI Root/Parent/SubCategoryViewer/Content/CategoryUnitParent";
const ITEMS: &str = "UI Root/Parent/ItemList/Content/Parent";
const DLG: &str = "__GameMain__/SystemUI Root/SystemDialog";
/// `TrialWearData.Save`: hair, skin, decorations and MPN 39..=69.
const TRIAL_BACKUP: [&str; 11] = ["hairf", "hairr", "hairt", "hairs", "skin", "acctatoo", "underhair", "hokuro", "lip", "eye", "eye_hi"];

#[derive(Clone, Debug, Default, PartialEq)]
struct State {
    next_label: String,
    maid: usize,
    /// Category buttons: (main, sub or empty, `Button` node).
    cat_buttons: Vec<(String, String, usize)>,
    main_cat: String,
    last_sub: BTreeMap<String, String>,
    /// Item units: (unit, buy button node, icon node, id).
    items: Vec<(usize, usize, usize, i32)>,
    selected: Option<i32>,
    /// `TrialWearData`: MPN → file before trial wear.
    trial_backup: Vec<(String, String)>,
    trial: bool,
    pending_buy: Option<i32>,
    wait_since: Option<u64>,
    wear_mode: BTreeMap<i64, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shop {
    wf: Manager,
    s: State,
}

fn lock<R>(game: &Game, f: impl FnOnce(&mut crate::tjs::TjsState) -> Result<R>) -> Result<R> {
    let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
    f(&mut tjs.runtime.host_mut().state)
}

/// `ItemData.is_have_item` / `ItemDataGroup.is_have_item`.
fn is_have(d: &GameData, p: &PlayerStatus, item: &ShopItem) -> bool {
    if !p.is_shop_lineup(item.id) {
        return false;
    }
    match &item.need_ids {
        None => p.shop_item_status(item.id) == Some(SHOP_PURCHASED),
        Some(_) => p.shop_item_status(item.id) == Some(SHOP_PURCHASED) || not_have(d, p, item).is_empty(),
    }
}

/// `ItemDataGroup.GetNotHaveItemData`.
fn not_have<'a>(d: &'a GameData, p: &PlayerStatus, item: &ShopItem) -> Vec<&'a ShopItem> {
    item.need_ids.iter().flatten().filter_map(|id| d.shop.get(id)).filter(|x| !is_have(d, p, x)).collect()
}

fn price(d: &GameData, p: &PlayerStatus, item: &ShopItem) -> i64 {
    match &item.need_ids {
        None => item.price.into(),
        Some(_) => not_have(d, p, item).iter().map(|x| i64::from(x.price)).sum(),
    }
}

fn menus(d: &GameData, p: &PlayerStatus, item: &ShopItem) -> Vec<String> {
    match &item.need_ids {
        None => item.menus.clone(),
        Some(_) => not_have(d, p, item).iter().flat_map(|x| x.menus.clone()).collect(),
    }
}

/// `trial_wear_item_menu_array`: the first menu, or every member's.
fn trial_menus(d: &GameData, item: &ShopItem) -> Vec<String> {
    match &item.need_ids {
        None => item.menus.first().cloned().into_iter().collect(),
        Some(ids) => ids.iter().filter_map(|id| d.shop.get(id)).flat_map(|x| x.menus.first().cloned()).collect(),
    }
}

/// `GameInShop.Start` + `GameInShopMain.Awake`.
pub fn start(game: &mut Game, opener: &mut Opener) -> Result<Shop> {
    let child = |name: &'static str, fade: f32| Child { name, root: ROOT.into(), fade_time: fade, status: FadeStatus::Null };
    let mut wf = Manager::new(game, vec![child("Main", 0.5), child("Move", 0.)]);
    let mut s = State::default();
    let tag = game.adv.tag_backup.clone();
    if tag.get("name").map(String::as_str) == Some("SceneShop") {
        s.next_label = tag.get("label").cloned().context("SceneCallにlabelの設定がされていませんでした")?;
    }
    let asm = game.managed(opener)?;
    s.wear_mode = crate::scene_edit::Tables::load(&asm)?.wear_mode;
    wf::set_click(game, &format!("{ROOT}/OK"), "GameInShopMain.OnClickOK")?;
    wf::set_click(game, &format!("{ROOT}/TrialWear"), "GameInShopMain.TrialWear")?;
    wf::set_click(game, &format!("{ROOT}/TrialWearReset"), "GameInShopMain.ResetTrialWear")?;
    let d = lock(game, |t| t.data())?;
    for (parent, sub) in [(MAIN_CATS, false), (SUB_CATS, true)] {
        let n = wf::node(game, parent)?;
        wf::clear_children(game, n);
        let _ = sub;
    }
    for (main, subs) in &d.shop_categories {
        let b = category_button(game, opener, MAIN_CATS, main)?;
        s.cat_buttons.push((main.clone(), String::new(), b));
        for sub in subs {
            let b = category_button(game, opener, SUB_CATS, sub)?;
            s.cat_buttons.push((main.clone(), sub.clone(), b));
        }
    }
    wf.call_screen(game, opener, &mut s, "Main")?;
    Ok(Shop { wf, s })
}

fn category_button(game: &mut Game, opener: &mut Opener, parent: &str, name: &str) -> Result<usize> {
    let p = wf::node(game, parent)?;
    let unit = wf::create_prefab(game, opener, p, "SceneShop/Prefab/CategoryButton")?;
    game.level.as_mut().context("shop level")?.scene.nodes[unit].name = name.into();
    let l = wf::child(game, unit, "Label")?;
    wf::state(game, l).text = Some(name.into());
    wf::child(game, unit, "Button")
}

pub fn update(game: &mut Game, opener: &mut Opener, sh: &mut Shop) -> Result<()> {
    sh.wf.update(game, opener, &mut sh.s)
}

impl State {
    fn lineup(&self, game: &Game) -> Result<BTreeMap<(String, String), Vec<i32>>> {
        lock(game, |t| {
            let d = t.data()?;
            let mut out: BTreeMap<(String, String), Vec<i32>> = BTreeMap::new();
            for (id, _) in t.player.shop_lineup.iter() {
                if let Some(item) = d.shop.get(id) {
                    out.entry((item.main_category.clone(), item.sub_category.clone())).or_default().push(*id);
                }
            }
            Ok(out)
        })
    }

    /// `OnClickFromMainCategoryButton`.
    fn select_main(&mut self, game: &mut Game, opener: &mut Opener, main: &str) -> Result<()> {
        self.main_cat = main.to_owned();
        let lineup = self.lineup(game)?;
        let mut first = None;
        for (m, sub, b) in self.cat_buttons.clone() {
            let unit = game.level.as_ref().unwrap().scene.nodes[b].parent.unwrap_or(b);
            if sub.is_empty() {
                wf::state(game, b).color = (m == main).then_some([1.; 4]);
                continue;
            }
            let on = m == main && lineup.get(&(m.clone(), sub.clone())).is_some_and(|v| !v.is_empty());
            wf::state(game, unit).active = Some(on);
            wf::state(game, b).color = None;
            if on {
                wf::state(game, b).click = Some(format!("GameInShopMain.OnClickFromSubCategoryButton:{sub}"));
                first.get_or_insert(sub.clone());
            }
        }
        wf::reposition(game, wf::node(game, SUB_CATS)?);
        let sub = self.last_sub.get(main).cloned().or(first);
        if let Some(sub) = sub {
            self.select_sub(game, opener, &sub)?;
        }
        Ok(())
    }

    /// `OnClickFromSubCategoryButton` → `SetItemList`.
    fn select_sub(&mut self, game: &mut Game, opener: &mut Opener, sub: &str) -> Result<()> {
        self.last_sub.insert(self.main_cat.clone(), sub.to_owned());
        for (m, s, b) in self.cat_buttons.clone() {
            if !s.is_empty() {
                wf::state(game, b).color = (m == self.main_cat && s == sub).then_some([1.; 4]);
            }
        }
        self.selected = None;
        wf::set_enabled(game, &format!("{ROOT}/TrialWear"), false)?;
        let parent = wf::node(game, ITEMS)?;
        wf::clear_children(game, parent);
        self.items.clear();
        let ids = self.lineup(game)?.remove(&(self.main_cat.clone(), sub.to_owned())).unwrap_or_default();
        let d = lock(game, |t| t.data())?;
        for id in ids {
            let Some(item) = d.shop.get(&id).cloned() else { continue };
            let unit = wf::create_prefab(game, opener, parent, "SceneShop/Prefab/ShopItem")?;
            let name = wf::child(game, unit, "ItemName")?;
            wf::state(game, name).text = Some(item.name.clone());
            let icon = wf::child(game, unit, "ItemIcon")?;
            if game.files.exists(&item.icon_file) {
                game.pending_textures.push((1, icon, item.icon_file.clone()));
            }
            wf::state(game, icon).click = Some("ShopItem.OnIconClick".into());
            if let Ok(f) = wf::child(game, icon, "Frame") {
                wf::state(game, f).active = Some(false);
            }
            let buy = wf::child(game, unit, "BuyButton")?;
            wf::state(game, buy).click = Some("ShopItem.OnClick".into());
            self.items.push((unit, buy, icon, id));
        }
        self.update_items(game)?;
        wf::reposition(game, parent);
        wf::reset_scroll(game, &format!("{ROOT}/ItemList/Content"))?;
        Ok(())
    }

    /// `ShopItem.UpdateItemData` of every unit.
    fn update_items(&mut self, game: &mut Game) -> Result<()> {
        let d = lock(game, |t| t.data())?;
        let player = lock(game, |t| Ok(t.player.clone()))?;
        for &(unit, buy, _, id) in &self.items.clone() {
            let Some(item) = d.shop.get(&id) else { continue };
            let have = is_have(&d, &player, item);
            let price_v = price(&d, &player, item);
            let p = wf::child(game, unit, "Price")?;
            let st = wf::state(game, p);
            st.text = Some(crate::daily::grouped(price_v));
            st.color = Some(if !have && player.money - price_v < 0 { [1., 0., 0., 1.] } else { [1.; 4] });
            wf::state(game, buy).active = Some(!have);
            wf::state(game, buy).enabled = Some(0 <= player.money - price_v);
            let pur = wf::child(game, unit, "Purchased")?;
            wf::state(game, pur).active = Some(have);
        }
        Ok(())
    }

    /// `OnItemClick`: pay, mark purchased, add the menus, set cards.
    fn buy(&mut self, game: &mut Game, id: i32) -> Result<()> {
        game.sound_ops.push(SoundOp::Play { kind: Kind::System, file: "SE025.ogg".into(), fade_s: 0., looping: false });
        lock(game, |t| {
            let d = t.data()?;
            let item = d.shop.get(&id).cloned().context("shop item")?;
            let pr = price(&d, &t.player, &item);
            let ms = menus(&d, &t.player, &item);
            let mut p = crate::maid_param::PlayerParam { st: &mut t.player, data: &d };
            p.add_money(-pr);
            p.add_shop_use_money(pr);
            if 100_000_000 <= p.st.shop_use_money {
                p.add_have_trophy(90);
            }
            for n in item.need_ids.iter().flatten() {
                p.set_shop_lineup(*n, SHOP_PURCHASED);
            }
            p.set_shop_lineup(id, SHOP_PURCHASED);
            for m in ms {
                p.add_have_item(&m)?;
            }
            check_card_set(&mut p)?;
            Ok(())
        })?;
        self.update_items(game)?;
        crate::daily::update_status_viewers(game)
    }

    /// `TrialWear`.
    fn trial_wear(&mut self, game: &mut Game, opener: &mut Opener) -> Result<()> {
        let Some(id) = self.selected else { return Ok(()) };
        let d = lock(game, |t| t.data())?;
        let Some(item) = d.shop.get(&id).cloned() else { return Ok(()) };
        let storage = opener.storage.clone();
        let m = self.maid;
        if self.trial_backup.is_empty() {
            let c = &game.characters[m];
            let mut names: Vec<String> = TRIAL_BACKUP.iter().map(|s| (*s).to_owned()).collect();
            names.extend((39..=69).filter_map(|i| crate::chara::MPN.get(i)).map(|s| (*s).to_owned()));
            self.trial_backup = names.into_iter().map(|n| { let f = c.file(&n).unwrap_or("").to_owned(); (n, f) }).collect();
        }
        for menu in trial_menus(&d, &item) {
            let bytes = game.files.read(storage.as_ref(), &menu).with_context(|| format!("{}\nnot file open.", item.name))?;
            let mf = crate::menu::MenuFile::parse(&bytes)?;
            game.characters[m].set_prop_file(&mf.category, &menu, false)?;
            if item.need_ids.is_none() {
                // SceneEditInfo.m_dicPartsTypeWearMode of the item's MPN.
                if let Some(mode) = crate::chara::mpn(&mf.category).and_then(|k| self.wear_mode.get(&(k as i64))).cloned() {
                    game.characters[m].set_mask_mode(&mode)?;
                }
            }
        }
        if item.need_ids.is_some() {
            let mode = if item.mask_mode.is_empty() { "None".to_owned() } else { item.mask_mode.clone() };
            game.characters[m].set_mask_mode(&mode)?;
        }
        game.characters[m].all_proc_prop(&game.files.clone(), storage.as_ref())?;
        self.trial = true;
        wf::set_enabled(game, &format!("{ROOT}/TrialWearReset"), true)?;
        Ok(())
    }

    /// `ResetTrialWear`.
    fn reset_trial(&mut self, game: &mut Game, opener: &mut Opener) -> Result<()> {
        wf::set_enabled(game, &format!("{ROOT}/TrialWearReset"), false)?;
        let m = self.maid;
        for (n, f) in std::mem::take(&mut self.trial_backup) {
            game.characters[m].set_prop_file(&n, &f, false)?;
        }
        let storage = opener.storage.clone();
        game.characters[m].all_proc_prop(&game.files.clone(), storage.as_ref())?;
        game.characters[m].set_mask_mode("None")?;
        self.trial = false;
        Ok(())
    }
}

/// `GameInShopMain.CheckCardSet`: complete purchased sets add their menus.
fn check_card_set(p: &mut crate::maid_param::PlayerParam) -> Result<()> {
    for (menus, ids) in &p.data.set_cards {
        if menus.iter().all(|m| p.st.have_item.contains(m)) {
            continue;
        }
        if ids.iter().all(|id| p.st.is_shop_lineup(*id) && p.st.shop_item_status(*id) == Some(SHOP_PURCHASED)) {
            for m in menus {
                p.add_have_item(m)?;
            }
        }
    }
    Ok(())
}

impl Screens for State {
    fn on_call(&mut self, _wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        if name != "Main" {
            return Ok(());
        }
        wf::set_active(game, &format!("{ROOT}/StatusViewer/LapsedDays"), false).ok();
        wf::set_active(game, &format!("{ROOT}/StatusViewer/SalonStatus"), false).ok();
        lock(game, |t| {
            let d = t.data()?;
            let mut p = crate::maid_param::PlayerParam { st: &mut t.player, data: &d };
            check_card_set(&mut p)?;
            p.check_salon_grade_shop_lineup();
            // Shop.CheckGroupItemData.
            for (id, item) in d.shop.iter() {
                if let Some(need) = &item.need_ids {
                    if !p.st.is_shop_lineup(*id) && need.iter().all(|n| p.st.is_shop_lineup(*n)) {
                        p.set_shop_lineup(*id, SHOP_NEW);
                    }
                }
            }
            Ok(())
        })?;
        game.adv_ui(AdvUi::SetBg("DressRoom_NoMirror".into()), opener)?;
        // Maid 0 if she is the leader, else the leader (or the first maid).
        let m0 = game.active_maids[0];
        let leader = lock(game, |t| Ok(t.leader().or_else(|| t.stock.first().copied())))?;
        let is_leader = m0.is_some_and(|m| lock(game, |t| Ok(t.maids.get(&m).is_some_and(|s| s.leader))).unwrap_or(false));
        self.maid = if is_leader { m0.unwrap() } else { leader.context("no stock maid")? };
        game.active_maids[0] = Some(self.maid);
        game.characters[self.maid].visible = true;
        game.adv_ui(AdvUi::AllProcProp { man: false, slot: 0 }, opener)?;
        self.trial_backup.clear();
        self.selected = None;
        wf::set_enabled(game, &format!("{ROOT}/TrialWear"), false)?;
        wf::set_enabled(game, &format!("{ROOT}/TrialWearReset"), false)?;
        // Main categories with lineup items; the first is selected.
        let lineup = self.lineup(game)?;
        let mut first = None;
        for (m, sub, b) in self.cat_buttons.clone() {
            let unit = game.level.as_ref().unwrap().scene.nodes[b].parent.unwrap_or(b);
            if sub.is_empty() {
                let on = lineup.iter().any(|(k, v)| k.0 == m && !v.is_empty());
                wf::state(game, unit).active = Some(on);
                if on {
                    wf::state(game, b).click = Some(format!("GameInShopMain.OnClickFromMainCategoryButton:{m}"));
                    first.get_or_insert(m.clone());
                }
            } else {
                wf::state(game, unit).active = Some(false);
            }
        }
        wf::reposition(game, wf::node(game, MAIN_CATS)?);
        if let Some(m) = first {
            self.select_main(game, opener, &m)?;
        }
        self.wait_since = None;
        game.adv_ui(AdvUi::LightMain { rotation: None, intensity: None }, opener)?;
        game.adv_ui(AdvUi::AllResetPos, opener)?;
        game.adv_ui(AdvUi::CameraReset, opener)?;
        Ok(())
    }

    fn is_call_fade_in(&mut self, _wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<bool> {
        if name != "Main" {
            return Ok(true);
        }
        if game.characters[self.maid].busy() {
            return Ok(false);
        }
        match self.wait_since {
            None => {
                self.wait_since = Some(game.now);
                game.adv_ui(AdvUi::MotionScript { slot: 0, file: "h_kaiwa_tati.ks".into(), label: "*会話立ち待機".into() }, opener)?;
                let c = &mut game.characters[self.maid];
                let sets = c.blend_sets.clone();
                c.face.anime("通常", &sets);
                c.face.blend("無し");
                Ok(false)
            }
            Some(t) if game.now < t + 500 => Ok(false),
            Some(_) => {
                game.adv_ui(AdvUi::Eye { man: false, slot: 0, look: EyeLook::Camera("目と顔を向ける".into()), blend_s: 0.5 }, opener)?;
                // Head bone − offset: the standing head height stands in for
                // the posed bone (SATD-cm3d2-game-logic, CPU skeleton).
                let p = game.characters[self.maid].position;
                let target = [p[0] + 0.01597595, p[1] + 1.45 - 0.22453693, p[2] + 0.043105565];
                game.adv_ui(AdvUi::Camera { target, distance: 1.6, around: [173.7126, 9.999997] }, opener)?;
                game.camera_offset = crate::ngui::WIDTH / 256. * 181. - crate::ngui::WIDTH / 2.;
                game.yotogi_saved_camera = Some(game.camera);
                game.sound_ops.push(SoundOp::Play { kind: Kind::Bgm, file: "BGM016.ogg".into(), fade_s: 0.5, looping: true });
                Ok(true)
            }
        }
    }

    fn on_finish(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        match name {
            "Main" => {
                if self.trial {
                    self.reset_trial(game, opener)?;
                }
            }
            "Move" => {
                let label = std::mem::take(&mut self.next_label);
                if !label.is_empty() {
                    let storage = opener.storage.clone();
                    game.adv.jump_label(&game.files, storage.as_ref(), &label)?;
                    game.adv.exec();
                }
            }
            _ => {}
        }
        let _ = wf;
        Ok(())
    }

    fn update(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        let status = wf.status(name);
        if name == "Move" && status == FadeStatus::Wait {
            wf.finish(game, opener, self, "Move")?;
            return Ok(());
        }
        if name != "Main" {
            return Ok(());
        }
        if status == FadeStatus::FadeEnd && !game.characters[self.maid].busy() {
            // InitMaidAndCamera(false), then Move.
            game.stop_motion_scripts();
            game.characters[self.maid].visible = false;
            game.camera_offset = 0.;
            game.adv_ui(AdvUi::AllResetPos, opener)?;
            wf.call_screen(game, opener, self, "Move")?;
            return Ok(());
        }
        if status != FadeStatus::Wait {
            return Ok(());
        }
        for (_, node, handler) in std::mem::take(&mut game.events) {
            let (method, arg) = handler.split_once(':').map_or((handler.as_str(), None), |(m, a)| (m, Some(a)));
            match method {
                "GameInShopMain.OnClickFromMainCategoryButton" => self.select_main(game, opener, arg.unwrap_or(""))?,
                "GameInShopMain.OnClickFromSubCategoryButton" => self.select_sub(game, opener, arg.unwrap_or(""))?,
                "ShopItem.OnIconClick" => {
                    let Some(&(_, _, _, id)) = self.items.iter().find(|i| i.2 == node) else { continue };
                    self.selected = if self.selected == Some(id) { None } else { Some(id) };
                    for &(_, _, icon, k) in &self.items.clone() {
                        if let Ok(f) = wf::child(game, icon, "Frame") {
                            wf::state(game, f).active = Some(self.selected == Some(k));
                        }
                    }
                    wf::set_enabled(game, &format!("{ROOT}/TrialWear"), self.selected.is_some())?;
                }
                "ShopItem.OnClick" => {
                    let Some(&(_, _, _, id)) = self.items.iter().find(|i| i.1 == node) else { continue };
                    let (name, pr, money) = lock(game, |t| {
                        let d = t.data()?;
                        let item = d.shop.get(&id).cloned().context("shop item")?;
                        Ok((item.name.clone(), price(&d, &t.player, &item), t.player.money))
                    })?;
                    self.pending_buy = Some(id);
                    if money - pr < 0 {
                        // The original's OK still runs CallOnClickEvent.
                        game.show_system_dialog("資金が不足しています。", "ShopItem.CallOnClickEvent")?;
                    } else {
                        game.show_system_dialog(&format!("{name}\nを購入しますか?\n資金 -{}CR", crate::daily::grouped(pr)), "ShopItem.CallOnClickEvent")?;
                        game.set_main_active(&format!("{DLG}/Base/Cancel"), true)?;
                        if let Some(n) = game.main.scene.find(&format!("{DLG}/Base/Cancel")) {
                            game.main.states.entry(n).or_default().click = Some("ShopItem.CloseDialog".into());
                        }
                    }
                }
                "ShopItem.CallOnClickEvent" => {
                    game.set_main_active(DLG, false)?;
                    if let Some(id) = self.pending_buy.take() {
                        self.buy(game, id)?;
                        let name = lock(game, |t| Ok(t.data()?.shop.get(&id).map(|i| i.name.clone()).unwrap_or_default()))?;
                        game.show_system_dialog(&format!("{name}を購入しました"), "ShopItem.CloseDialog")?;
                    }
                }
                "ShopItem.CloseDialog" => {
                    game.set_main_active(DLG, false)?;
                    self.pending_buy = None;
                }
                "GameInShopMain.TrialWear" => self.trial_wear(game, opener)?,
                "GameInShopMain.ResetTrialWear" => self.reset_trial(game, opener)?,
                "GameInShopMain.OnClickOK" if !game.characters[self.maid].busy() => {
                    wf.finish(game, opener, self, "Main")?;
                }
                _ => game.log.push(format!("SceneShop event {handler} not ported")),
            }
        }
        Ok(())
    }
}
