// SPDX-License-Identifier: MPL-2.0
//! Read-only room and type lists on the standalone facility canvas.
use super::*;

pub(super) const MENU: &str = "/Window Main Menu";
pub(super) const MINI: &str = "/Image Facility Mini Info";
pub(super) const RECIPES: &str = "/Window PowerUp List";
pub(super) const RECIPE_VIEW: &str = "/Window PowerUp List/Scroll View/Viewport";
const CARD: &str = "/Window PowerUp List/Scroll View/Viewport/Content/temp Image Recipe Info";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Tab {
    Build,
    Current,
    Catalog,
    Wardrobe,
}
impl Tab {
    pub(super) fn for_path(path: &str) -> Option<Self> {
        match path
            .strip_prefix("/Window Main Menu/Button List/")?
            .split('/')
            .next()?
        {
            "Toggle Facility Create and Destroy" => Some(Self::Build),
            "Toggle Facility Facility" => Some(Self::Wardrobe),
            "Toggle Facility List" => Some(Self::Current),
            "Toggle Facility Enable List" => Some(Self::Catalog),
            _ => None,
        }
    }
}

impl Screen {
    pub(super) fn show_information(
        &mut self,
        s: &mut TjsState,
        id: i32,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        out: &mut Vec<Widget>,
    ) -> Result<()> {
        let def = self.catalog.get(id)?;
        let room = self
            .inspected
            .and_then(|slot| s.retail.facility_index(slot));
        let current = if self.tab == Tab::Wardrobe
            && self
                .inspected
                .is_some_and(|slot| s.retail.construction.contains(&slot))
        {
            Some(upgrade::planned_info(s, id)?)
        } else {
            room.map(|room| upgrade::info(s, room)).transpose()?
        };
        for mut w in self.layout.instantiate(MINI, self.layout.rect(MINI)?)? {
            if w.path.contains("/Button Change Name") {
                continue;
            }
            if w.path.ends_with("/RawImage") {
                let count = if w.path.contains("parent stars Rank/") {
                    def.rank
                } else {
                    current.as_ref().map_or(0, |v| v.staff_rank)
                };
                for i in 0..count {
                    let mut star = w.clone();
                    star.rect = self.layout.cell(&w.path, i, count)?;
                    out.push(star);
                }
                continue;
            }
            if w.path == MINI {
                assets.facility_thumbnail(
                    op,
                    host,
                    c,
                    &mut w,
                    if current.is_some() {
                        "myroom_info_plate"
                    } else {
                        "myroom_info_plate2"
                    },
                )?;
            } else if w.path.ends_with("/Image Facility Thumbnail") {
                assets.facility_thumbnail(op, host, c, &mut w, &def.thumbnail)?;
            } else if w.label.is_some() {
                let value = match w.path.rsplit('/').next().unwrap_or("") {
                    "Text" if self.tab == Tab::Wardrobe => self
                        .catalog
                        .display_name(&s.retail, self.inspected.context("missing room")?)?
                        .into(),
                    "Text" => current
                        .as_ref()
                        .map_or_else(|| def.name.clone(), |v| v.name.clone()),
                    "Text Level of facility (1)" => Catalog::level(&s.retail, id).to_string(),
                    "Text Status (1)" => {
                        if let Some(v) = &current {
                            v.status.to_owned()
                        } else {
                            continue;
                        }
                    }
                    "Text Number of people (1)" => {
                        if let Some(v) = &current {
                            v.staff.to_string()
                        } else {
                            continue;
                        }
                    }
                    "Text Number of min people" if current.is_none() => def.minimum.to_string(),
                    "Text Number of max people" if current.is_none() => def.maximum.to_string(),
                    "Text Number of cost" if current.is_none() => def.cost.to_string(),
                    _ => continue,
                };
                text(&mut w, &value);
            }
            out.push(w);
        }
        if self.tab == Tab::Wardrobe {
            return self.wardrobe_widgets(s, assets, op, host, c, out);
        }
        let recipes = upgrade::recipes_for_type(s, id)?;
        let first = self.layout.cell(CARD, 0, 1)?;
        let step = self.layout.cell(CARD, 1, 2)?[1] - first[1];
        ensure!(step > 0., "invalid facility recipe grid");
        let view = self.layout.rect(RECIPE_VIEW)?;
        let visible =
            (1. + ((view[1] + view[3] - first[1] - first[3]) / step).floor()).max(1.) as usize;
        let count = recipes.len().max(1);
        self.recipe_scroll = self.recipe_scroll.min(count.saturating_sub(visible));
        for w in self
            .layout
            .base
            .iter()
            .filter(|w| w.path.starts_with(RECIPES) && !w.path.starts_with(CARD))
        {
            if w.path.ends_with("/Button Slider Movement") {
                self.bindings.push((
                    w.rect,
                    Action::ScrollRecipes(w.rect[1] > view[1] + view[3] / 2.),
                ));
            }
            out.push(w.clone());
        }
        for (row, i) in (self.recipe_scroll..count).take(visible).enumerate() {
            let mut rect = first;
            rect[1] += row as f32 * step;
            let recipe = recipes.get(i);
            for mut w in self.layout.instantiate(CARD, rect)? {
                if w.path.contains("/parent ") {
                    continue;
                }
                if w.path.ends_with("/Text Recipe Name") {
                    text(&mut w, recipe.map_or("レシピ無し", |r| &r.name));
                }
                if w.path.ends_with("/Text Recipe Number") {
                    text(
                        &mut w,
                        &recipe.map_or(String::new(), |_| format!("RECIPE {}", i + 1)),
                    );
                }
                out.push(w);
            }
            if let Some(recipe) = recipe {
                let origin = self.layout.rect(CARD)?;
                self.recipe_fields(
                    s,
                    room,
                    recipe,
                    [rect[0] - origin[0], rect[1] - origin[1]],
                    out,
                )?;
            }
        }
        Ok(())
    }
    fn recipe_fields(
        &self,
        s: &mut TjsState,
        room: Option<usize>,
        recipe: &upgrade::Recipe,
        delta: [f32; 2],
        out: &mut Vec<Widget>,
    ) -> Result<()> {
        let materials = room
            .map(|i| upgrade::previous(s, i, recipe.id).to_vec())
            .unwrap_or_default();
        let categories = s.retail.table("facility_material_category.nei")?;
        let names = s.retail.table("facility_material.nei")?;
        for (i, &cat) in recipe.categories.iter().enumerate() {
            let row = (1..categories.rows)
                .find(|&r| categories.int(0, r) == cat)
                .context("unknown facility material category")?;
            let mat = materials.get(i).copied().unwrap_or(0);
            let name = (1..names.rows)
                .find(|&r| names.exists(0, r) && names.int(0, r) == mat)
                .map(|r| names.string(1, r))
                .unwrap_or_default();
            self.field(
                "parent materials/temp item",
                i,
                recipe.categories.len(),
                delta,
                &[
                    ("Text Category Name", categories.string(1, row)),
                    ("Text Category Number", format!("CATEGORY {}", i + 1)),
                    ("Text Material Name", name),
                ],
                out,
            )?;
        }
        for (i, condition) in recipe.conditions.iter().enumerate() {
            self.field(
                "parent conditions/temp item",
                i,
                recipe.conditions.len(),
                delta,
                &[
                    ("Text Conditions", condition.clone()),
                    ("Text Number", (i + 1).to_string()),
                ],
                out,
            )?;
        }
        let params = s.retail.table("facility_parameters.nei")?;
        for (group, values) in [
            ("parent default parameter/GameObject", recipe.base),
            (
                "parent additional parameter/GameObject",
                upgrade::additional(&mut s.retail, &materials)?,
            ),
        ] {
            let shown = values
                .iter()
                .enumerate()
                .filter(|(_, v)| **v > 0)
                .collect::<Vec<_>>();
            for (i, (col, value)) in shown.iter().enumerate() {
                self.field(
                    group,
                    i,
                    shown.len(),
                    delta,
                    &[(
                        "item Text Parameter",
                        format!("{}  +{value}", params.string(1, col + 1)),
                    )],
                    out,
                )?;
            }
        }
        Ok(())
    }
    fn field(
        &self,
        group: &str,
        index: usize,
        count: usize,
        delta: [f32; 2],
        labels: &[(&str, String)],
        out: &mut Vec<Widget>,
    ) -> Result<()> {
        let path = format!("{CARD}/{group}");
        let mut rect = self.layout.cell(&path, index, count)?;
        rect[0] += delta[0];
        rect[1] += delta[1];
        for mut w in self.layout.instantiate(&path, rect)? {
            if let Some((_, v)) = labels
                .iter()
                .find(|(name, _)| w.path.rsplit('/').next() == Some(*name))
            {
                text(&mut w, v);
            }
            out.push(w);
        }
        Ok(())
    }
}
