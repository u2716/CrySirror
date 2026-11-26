// SPDX-License-Identifier: MPL-2.0
//! Shop.CreateData, GameInShopMain.OnCall and OnItemClick in the standalone player.
use super::*;
use crate::params::{SHOP_NEW, SHOP_PURCHASED};

enum Product {
    Parts(Vec<String>),
    Group(Vec<i32>),
    Event {
        flag: String,
        overlaps: Vec<i32>,
        material: Option<i32>,
    },
}

pub(crate) struct Item {
    pub name: String,
    pub icon: String,
    pub description: String,
    pub mask_mode: String,
    category: (String, String),
    backgrounds: Vec<i32>,
    price: i64,
    product: Product,
}

#[derive(Default)]
pub(crate) struct Catalog {
    pub items: BTreeMap<i32, Item>,
    pub categories: Vec<(String, String)>,
    pub visible: Vec<i32>,
    fitting_tabs: BTreeSet<String>,
    cards: Vec<(Vec<String>, Vec<i32>)>,
}

fn ids(cell: String) -> Result<Vec<i32>> {
    cell.split(',')
        .filter(|v| !v.trim().is_empty())
        .map(|v| Ok(v.trim().parse()?))
        .collect()
}

impl Catalog {
    pub fn open(s: &mut TjsState, background: i32) -> Result<Self> {
        let mut shop = Self::default();
        s.retail.index_parts()?;
        let mut lineup = s.player.shop_lineup.clone();
        let grade = s.retail.table("game_in_shop_clubgrade_setting.nei")?;
        let buildings = s.retail.table("game_in_shop_building_setting.nei")?;
        let mut unlock = |id| {
            if !lineup.contains(&id) {
                lineup.insert(id, SHOP_NEW);
            }
        };
        for r in (1..grade.rows)
            .filter(|&r| grade.exists(0, r))
            .take(s.player.current_salon_grade.max(0) as usize)
        {
            for id in ids(grade.string(1, r))? {
                unlock(id);
            }
        }
        for r in (1..buildings.rows).filter(|&r| buildings.exists(0, r)) {
            if ids(buildings.string(0, r))?
                .iter()
                .any(|id| s.retail.buildings.as_ref().is_some_and(|v| v.contains(id)))
            {
                for id in ids(buildings.string(1, r))? {
                    unlock(id);
                }
            }
        }
        for (filename, kind) in [
            ("game_in_shop_item_setting.nei", 0),
            ("game_in_shop_item_groups_setting.nei", 1),
            ("game_in_shop_event_item_setting.nei", 2),
        ] {
            if kind == 2 && !s.retail.source()?.files.exists(filename) {
                continue;
            }
            let t = s.retail.table(filename)?;
            for r in (1..t.rows).filter(|&r| t.exists(0, r)) {
                let id = t.int(0, r);
                if !s.retail.enabled("game_in_shop_itemlist", id)? {
                    continue;
                }
                let (cat, price, product) = match kind {
                    0 => (
                        6,
                        t.int(8, r),
                        Product::Parts(
                            (9..t.columns)
                                .filter(|&c| t.exists(c, r))
                                .map(|c| format!("{}.menu", t.string(c, r).to_lowercase()))
                                .collect(),
                        ),
                    ),
                    1 => {
                        let members: Vec<_> = (8..t.columns)
                            .filter(|&c| t.exists(c, r))
                            .map(|c| t.int(c, r))
                            .collect();
                        if !lineup.contains(&id) && members.iter().all(|m| lineup.contains(m)) {
                            lineup.insert(id, SHOP_NEW);
                        }
                        (6, 0, Product::Group(members))
                    }
                    _ => (
                        5,
                        t.int(7, r),
                        Product::Event {
                            flag: t.string(8, r).to_lowercase(),
                            overlaps: ids(t.string(9, r))?
                                .into_iter()
                                .filter(|&v| v != id)
                                .collect(),
                            material: if t.string(10, r) == "建物強化素材" && t.exists(11, r)
                            {
                                Some(t.string(11, r).trim().parse()?)
                            } else {
                                None
                            },
                        },
                    ),
                };
                let mode = t.string(5, r);
                let item = Item {
                    mask_mode: if kind == 2 || mode.is_empty() {
                        "None".into()
                    } else {
                        mode
                    },
                    name: t.string(2, r),
                    icon: t.string(3, r),
                    description: t.string(4, r).replace("《改行》", "\n"),
                    category: (t.string(cat, r), t.string(cat + 1, r)),
                    backgrounds: ids(t.string(1, r))?,
                    price: i64::from(price),
                    product,
                };
                ensure!(
                    shop.items.insert(id, item).is_none(),
                    "duplicate shop item {id}"
                );
            }
        }
        let cards = s.retail.table("game_in_shop_setcard_setting.nei")?;
        for r in (1..cards.rows).filter(|&r| cards.exists(0, r)) {
            let need: Vec<_> = (1..cards.columns)
                .take_while(|&c| cards.exists(c, r))
                .map(|c| cards.int(c, r))
                .collect();
            if !need.is_empty() && need.iter().all(|id| shop.items.contains_key(id)) {
                shop.cards.push((
                    cards
                        .string(0, r)
                        .split(',')
                        .map(|v| format!("{}.menu", v.to_lowercase()))
                        .collect(),
                    need,
                ));
            }
        }
        shop.visible = lineup
            .iter()
            .filter(|(id, _)| {
                shop.items
                    .get(id)
                    .is_some_and(|it| it.backgrounds.contains(&background))
            })
            .map(|(&id, _)| id)
            .collect();
        let categories = s.retail.table("game_in_shop_category_list.nei")?;
        for r in (1..categories.rows).filter(|&r| categories.exists(0, r)) {
            let cat = (categories.string(0, r), categories.string(1, r));
            if shop.visible.iter().any(|id| shop.items[id].category == cat)
                && !shop.categories.contains(&cat)
            {
                if categories.string(2, r) == "〇" {
                    shop.fitting_tabs.insert(cat.1.clone());
                }
                shop.categories.push(cat);
            }
        }
        s.player.shop_lineup = lineup;
        shop.unlock_cards(s);
        Ok(shop)
    }

    pub fn in_category(&self, cat: &(String, String)) -> Vec<i32> {
        self.visible
            .iter()
            .copied()
            .filter(|id| self.items[id].category == *cat)
            .collect()
    }

    pub fn fitting_tab(&self, sub: &str) -> bool {
        self.fitting_tabs.contains(sub)
    }
    pub fn parts(&self, id: i32) -> bool {
        !matches!(self.items[&id].product, Product::Event { .. })
    }
    pub fn preview_menus(&self, id: i32) -> Result<Vec<&str>> {
        let first = |member: &i32| -> Result<&str> {
            let Some(Product::Parts(names)) = self.items.get(member).map(|i| &i.product) else {
                bail!("invalid fitting item {member}")
            };
            names
                .first()
                .map(String::as_str)
                .context("missing fitting menu")
        };
        match &self.items[&id].product {
            Product::Parts(_) => Ok(vec![first(&id)?]),
            Product::Group(m) => m.iter().map(first).collect(),
            Product::Event { .. } => bail!("event item has no fitting menu"),
        }
    }

    fn purchased(s: &TjsState, id: i32) -> bool {
        s.player.shop_lineup.get(&id) == Some(&SHOP_PURCHASED)
    }

    fn unlock_cards(&self, s: &mut TjsState) {
        for (menus, ids) in &self.cards {
            if ids.iter().all(|&id| Self::purchased(s, id)) {
                for menu in menus {
                    s.retail.owned_parts.insert(menu.clone(), true);
                }
            }
        }
    }

    pub fn owns(&self, s: &TjsState, id: i32) -> bool {
        match &self.items[&id].product {
            Product::Event { flag, .. } => s.player.have_item.get(flag) == Some(&true),
            Product::Group(m) => {
                s.player.shop_lineup.contains(&id)
                    && (Self::purchased(s, id)
                        || m.iter().all(|id| {
                            !s.player.shop_lineup.contains(id) || Self::purchased(s, *id)
                        }))
            }
            Product::Parts(_) => Self::purchased(s, id),
        }
    }

    pub fn eligible(&self, s: &TjsState, id: i32) -> bool {
        match &self.items[&id].product {
            Product::Event { overlaps, .. } => !overlaps
                .iter()
                .any(|id| self.items.contains_key(id) && self.owns(s, *id)),
            _ => true,
        }
    }

    pub fn price(&self, s: &TjsState, id: i32) -> Result<i64> {
        match &self.items[&id].product {
            Product::Group(m) => m
                .iter()
                .filter(|id| s.player.shop_lineup.contains(*id) && !Self::purchased(s, **id))
                .map(|id| {
                    self.items
                        .get(id)
                        .map(|p| p.price)
                        .context("missing shop group member")
                })
                .sum(),
            _ => Ok(self.items[&id].price),
        }
    }

    pub fn buy(&self, s: &mut TjsState, id: i32) -> Result<()> {
        ensure!(
            self.visible.contains(&id) && !self.owns(s, id) && self.eligible(s, id),
            "shop item unavailable: {id}"
        );
        let price = self.price(s, id)?;
        ensure!(
            price >= 0 && s.player.money >= price,
            "insufficient shop funds"
        );
        let total = s
            .player
            .shop_use_money
            .checked_add(price)
            .context("total purchase price overflow")?;
        match &self.items[&id].product {
            Product::Event { flag, material, .. } => {
                s.player.have_item.insert(flag.clone(), true);
                if let Some(material) = material {
                    s.retail.materials.insert(*material);
                }
            }
            product => {
                let members = match product {
                    Product::Group(m) => m.as_slice(),
                    _ => std::slice::from_ref(&id),
                };
                let mut menus = Vec::new();
                for member in members
                    .iter()
                    .filter(|id| s.player.shop_lineup.contains(*id) && !Self::purchased(s, **id))
                {
                    let Some(Product::Parts(parts)) = self.items.get(member).map(|i| &i.product)
                    else {
                        bail!("invalid shop group member {member}")
                    };
                    menus.extend(parts.iter().cloned());
                }
                for menu in menus {
                    s.retail.owned_parts.insert(menu, true);
                }
                for member in members {
                    if s.player.shop_lineup.contains(member) {
                        s.player.shop_lineup.insert(*member, SHOP_PURCHASED);
                    }
                }
                s.player.shop_lineup.insert(id, SHOP_PURCHASED);
                self.unlock_cards(s);
            }
        }
        s.player.money -= price;
        s.player.shop_use_money = total;
        Ok(())
    }
}
