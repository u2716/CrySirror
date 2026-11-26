// SPDX-License-Identifier: MPL-2.0
//! Script-visible state owned by the retail activities between scene calls.
use super::*;

const DANCE_TYPES: &[&str] = &[
    "Free",
    "Challenge",
    "VS",
    "Encore",
    "View",
    "BenchMark",
    "Touch",
    "VS_Skip",
];

#[derive(Clone)]
pub(super) struct Activities {
    dance_type: i32,
    bgm: String,
    presets: Vec<String>,
    karaoke: bool,
    second_dance: bool,
    capture_ok: bool,
    rank: String,
    candidates: std::collections::VecDeque<usize>,
    simplified: bool,
    sold_out: BTreeSet<i32>,
}

impl Default for Activities {
    fn default() -> Self {
        Self {
            dance_type: 4,
            bgm: String::new(),
            presets: Vec::new(),
            karaoke: false,
            second_dance: false,
            capture_ok: false,
            rank: String::new(),
            candidates: Default::default(),
            simplified: true,
            sold_out: Default::default(),
        }
    }
}

pub(super) fn set_dance(s: &mut TjsState, a: &[Variant]) -> Result<Variant> {
    arity(a, 1)?;
    let d = &mut s.retail.activities;
    d.bgm = arg_string(a, 0);
    d.presets = vec![String::new()];
    d.karaoke = true;
    d.second_dance = false;
    d.capture_ok = false;
    s.tmp_flags.insert("ダンスOVRカメラタイプ".into(), 0);
    Ok(Variant::Void)
}

/// BaseKagManager.TagSetDanceType and the free dance menu.
pub(crate) fn set_mode(s:&mut TjsState,mode:&str)->Result<()> {
    let kind=enum_value(DANCE_TYPES,mode).with_context(||format!("unknown dance mode {mode}"))?;
    ensure!(matches!(mode,"Free"|"View"),"unsupported COM3D2 dance mode {mode}");
    let d=&mut s.retail.activities;
    d.dance_type=kind;
    d.second_dance=false;
    d.capture_ok=false;
    Ok(())
}

pub(crate) fn cast(s:&mut TjsState,slots:&[Option<usize>]){
    s.active_maids.fill(None);
    let mut count=0;
    for (slot,id) in slots.iter().enumerate(){
        if let Some(id)=id {facility::activate(s,slot,*id,false);count+=1;}
    }
    s.tmp_flags.insert("ダンス設定人数".into(),count);
}
pub(super) fn check_dance(s: &mut TjsState, a: &[Variant]) -> Result<Variant> {
    ensure!(!a.is_empty(), "expected at least one dance type");
    let mut matched = false;
    for i in 0..a.len() {
        let name = arg_string(a, i);
        let ty =
            enum_value(DANCE_TYPES, &name).with_context(|| format!("unknown dance type {name}"))?;
        matched |= ty == s.retail.activities.dance_type;
    }
    Ok(boolean(matched))
}

pub(super) fn challenge_rank(s: &mut TjsState, a: &[Variant]) -> Result<Variant> {
    ensure!(!a.is_empty(), "expected at least one challenge rank");
    Ok(boolean(
        (0..a.len()).any(|i| arg_string(a, i) == s.retail.activities.rank),
    ))
}

pub(super) fn create_show_loop(s: &mut TjsState, _: &[Variant]) -> Result<Variant> {
    s.stock_loop.clear();
    let show = &mut s.retail.activities;
    if show.simplified {
        s.stock_loop.extend(show.candidates.iter().copied());
    } else if let Some(maid) = show.candidates.pop_front() {
        s.stock_loop.push(maid);
    }
    Ok(int(s.stock_loop.len() as i64))
}

pub(super) fn show_count(s: &mut TjsState, _: &[Variant]) -> Result<Variant> {
    Ok(int(s.retail.activities.candidates.len() as i64))
}

pub(super) fn bought(s: &mut TjsState, a: &[Variant]) -> Result<Variant> {
    ensure!(!a.is_empty(), "expected at least one casino item ID");
    let t = s.retail.table("casinoshop_data.nei")?;
    for i in 0..a.len() {
        let id = arg_int(a, i) as i32;
        if !s.retail.activities.sold_out.contains(&id)
            || !s.retail.enabled("casinoshop_enable_id", id)?
            || !(1..t.rows).any(|r| t.exists(0, r) && t.int(0, r) == id)
        {
            return Ok(boolean(false));
        }
    }
    Ok(boolean(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dance_selection_replaces_previous_song_and_resets_capture() -> Result<()> {
        let mut t = super::super::tests::script()?;
        assert!(t.eval_bool("CheckDanceType('Touch','View') && GetChallengeRank('')")?);
        assert!(t.eval("CheckDanceType('View','unknown')").is_err());
        t.eval("SetDanceData('first')")?;
        let d = &mut t.runtime.host_mut().state.retail.activities;
        d.presets.push("prev".into());
        d.second_dance = true;
        d.capture_ok = true;
        d.dance_type = 1;
        d.rank = "S".into();
        t.exec(
            "restart",
            "SetTmpFlag('ダンスOVRカメラタイプ',2); SetDanceData('second');",
        )?;
        assert!(t.eval_bool("CheckDanceType('1') && !CheckDanceType('View') && GetChallengeRank('A','S') && GetTmpFlag('ダンスOVRカメラタイプ')==0")?);
        let d = &t.runtime.host().state.retail.activities;
        assert_eq!(d.bgm, "second");
        assert_eq!(d.presets, [""]);
        assert!(d.karaoke && !d.capture_ok && !d.second_dance);
        Ok(())
    }
    #[test]
    fn show_queue_copies_or_consumes_matching_roster() -> Result<()> {
        let mut t = super::super::tests::script()?;
        t.runtime.host_mut().state.retail.activities.candidates = [1, 0].into();
        assert!(
            t.eval_bool("CreateCompetitiveShowLoopData()==2 && GetCompetitiveShowLoopCount()==2")?
        );
        assert_eq!(t.runtime.host().state.stock_loop, [1, 0]);
        t.runtime.host_mut().state.retail.activities.simplified = false;
        assert!(
            t.eval_bool("CreateCompetitiveShowLoopData()==1 && GetCompetitiveShowLoopCount()==1")?
        );
        assert_eq!(t.runtime.host().state.stock_loop, [1]);
        assert!(t.eval_bool("CreateCompetitiveShowLoopData()==1 && !GetCompetitiveShowLoopCount() && !CreateCompetitiveShowLoopData()")?);
        assert!(t.runtime.host().state.stock_loop.is_empty());
        Ok(())
    }
    pub(in crate::tjs::retail) fn buy_for_test(s: &mut State, id: i32) {
        s.activities.sold_out.insert(id);
    }
}
#[cfg(test)]
pub(super) use tests::buy_for_test;

/// The standalone selection list excludes sub-characters.
pub(crate) fn entrants(s: &TjsState) -> Vec<usize> {
    let mut ids: Vec<_> = s.stock.iter().copied().filter(|id| {
        s.maids.get(id).is_some_and(|m| s.retail.characters.get(&m.guid_id).is_some_and(|x| x.sub_name.is_none()))
    }).collect();
    s.sort_maid_standard(&mut ids);
    ids
}

pub(crate) fn original_entrant(s:&TjsState,id:usize)->bool {
    s.maids.get(&id).and_then(|m|s.retail.characters.get(&m.guid_id)).is_some_and(|ch|ch.sub_name.is_none() && ch.old.is_none())
}
pub(crate) fn select_show(s: &mut TjsState, ids: &[usize]) {
    s.retail.show_in_progress=false;
    s.retail.activities.candidates = ids.iter().copied().collect();
    s.retail.activities.simplified=false;
}
pub(crate) fn simplify_show(s: &mut TjsState, simplified: bool) {
    s.retail.show_in_progress=true;
    s.retail.activities.simplified = simplified;
    s.tmp_flags.insert("品評会_演出省略".into(), i64::from(simplified));
}
pub(crate) fn show_candidates(s:&TjsState) -> Vec<usize> {
    s.retail.activities.candidates.iter().copied().collect()
}

pub(crate) fn casinos(s:&mut TjsState)->Result<Vec<(usize,String)>> {
    let t=s.retail.table("facility.nei")?;
    let mut result=Vec::new();
    for name in ["高級カジノ","カジノ"] {
        if let Some(r)=(1..t.rows).find(|&r|t.string(2,r)==name)
            && let Some((i,f))=s.retail.facilities.iter().enumerate().find(|(_,f)|f.type_id==t.int(0,r)) {
            result.push((f.slot.unwrap_or(i),f.name.clone().unwrap_or_else(||name.into())));
        }
    }
    Ok(result)
}
pub(crate) fn first_dealer(s:&mut TjsState)->Result<usize> {
    let t=s.retail.table("maid_status_personal_list.nei")?;
    let muku=t.int(0,stock::named(&t,1,"Muku")?);
    s.active_maids.iter().flatten().chain(s.stock.iter()).copied()
      .find(|id|s.retail.main_characters.contains(id)&&s.maids.get(id).is_some_and(|m|m.personal==muku))
      .context("casino requires the main Muku dealer")
}
