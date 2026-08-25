//! UI (P1-M3): HUB screen + battle HUD, driven by the mod content.
//!
//! Phase model (simple resource): HUB runs until Play is pressed; battle runs
//! until won/lost; then the HUB respawns (loot kept). The BUILD phase
//! (click-to-place) is spade's slice — the loop currently skips it.

use bevy::prelude::*;

use crate::content::Content;
use crate::sim::Battle;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Splash,
    Hub,
    Build,
    Battle,
}

#[derive(Resource, Default)]
pub struct GameState {
    pub phase: Phase,
}

#[derive(Component)]
pub struct HubRoot;
#[derive(Component)]
pub struct HudRoot;
#[derive(Component)]
pub struct PlayButton;
#[derive(Component)]
pub struct CurrencyLabel;
#[derive(Component)]
pub struct WeaponCard(pub String);
#[derive(Component)]
pub struct UpgradeRow(pub String);
#[derive(Component)]
pub struct StatusLabel;

pub struct UiPlugin2;

impl Plugin for UiPlugin2 {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameState>()
            .add_systems(Startup, seed_economy)
            .add_systems(
                Update,
                (handle_buttons, refresh_hub, refresh_hud, check_battle_end).chain(),
            );
    }
}

fn seed_economy(mut battle: ResMut<Battle>) {
    // Starting grant so the first weapon unlock is possible; everything else
    // is earned (user sign-off: orbs per wave milestone + map clear, cubes per
    // run score, diamonds = kills).
    if battle.orbs == 0 {
        battle.orbs = 1;
    }
}

fn root_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::FlexStart,
        padding: UiRect::all(Val::Px(16.0)),
        row_gap: Val::Px(10.0),
        ..default()
    }
}

fn panel_node() -> Node {
    Node {
        width: Val::Percent(80.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::FlexStart,
        padding: UiRect::all(Val::Px(12.0)),
        row_gap: Val::Px(6.0),
        ..default()
    }
}

fn label(s: &str, size: f32) -> (Text, TextFont, TextColor) {
    (
        Text::new(s),
        TextFont::from_font_size(size),
        TextColor(Color::srgb(0.9, 0.9, 0.92)),
    )
}

pub(crate) fn spawn_hub(mut commands: Commands, content: Res<Content>, battle: Res<Battle>) {
    let mut hub = commands.spawn((
        HubRoot,
        root_node(),
        BackgroundColor(Color::srgb(0.05, 0.06, 0.10)),
    ));
    hub.with_children(|p| {
        p.spawn(label("MOONSHELL — Sector 1 · Crater Rim", 30.0));
        p.spawn((
            CurrencyLabel,
            label(&format!("💎 {}   🟠 {}   🟪 {}", battle.diamonds, battle.orbs, battle.cubes), 20.0),
        ));

        p.spawn(panel_node()).with_children(|wp| {
            wp.spawn(label("WEAPONS — click to unlock (🟠 orbs)", 18.0));
            let mut weapons: Vec<_> = content.weapons.values().collect();
            weapons.sort_by(|a, b| a.cost_orbs.cmp(&b.cost_orbs));
            for w in weapons {
                wp.spawn((
                    WeaponCard(w.id.clone()),
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.13, 0.20)),
                    label(
                        &format!(
                            "{}  — {} orbs · dmg {} · rate {}/s · cap {}",
                            w.name, w.cost_orbs, w.damage, w.fire_rate, w.cap
                        ),
                        15.0,
                    ),
                ));
            }
        });

        p.spawn(panel_node()).with_children(|up| {
            up.spawn(label("UPGRADES — click to rank up (🟪 cubes)", 18.0));
            let mut upgrades: Vec<_> = content.upgrades.values().collect();
            upgrades.sort_by(|a, b| a.id.cmp(&b.id));
            for u in upgrades {
                up.spawn((
                    UpgradeRow(u.id.clone()),
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.13, 0.11, 0.22)),
                    label(
                        &format!("{}  — {} cubes/rank · max rank {}", u.id, u.cost.cubes, u.rank_max),
                        14.0,
                    ),
                ));
            }
        });

        p.spawn((
            PlayButton,
            Button,
            Node {
                width: Val::Px(240.0),
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.10, 0.45, 0.20)),
            label("▶ PLAY — Sector 1", 20.0),
        ));
    });
}

fn handle_buttons(
    cards: Query<(&Interaction, &WeaponCard)>,
    upgrades: Query<(&Interaction, &UpgradeRow)>,
    play: Query<&Interaction, (With<PlayButton>, Changed<Interaction>)>,
    mut battle: ResMut<Battle>,
    mut state: ResMut<GameState>,
    hub: Query<Entity, With<HubRoot>>,
    mut commands: Commands,
) {
    for (inter, card) in &cards {
        if *inter == Interaction::Pressed && !battle.owned_weapons.contains(&card.0) {
            let cost = battle.weapon_prices.get(&card.0).copied().unwrap_or(u32::MAX);
            if battle.orbs >= cost {
                battle.orbs -= cost;
                battle.owned_weapons.push(card.0.clone());
            }
        }
    }
    for (inter, row) in &upgrades {
        if *inter == Interaction::Pressed && battle.cubes > 0 {
            battle.cubes -= 1;
            *battle.upgrade_ranks.entry(row.0.clone()).or_insert(0) += 1;
        }
    }
    for inter in &play {
        if *inter == Interaction::Pressed {
            state.phase = Phase::Battle;
            if let Ok(e) = hub.single() {
                commands.entity(e).despawn();
            }
        }
    }
}

fn refresh_hub(
    battle: Res<Battle>,
    state: Res<GameState>,
    mut currency: Query<&mut Text, With<CurrencyLabel>>,
    mut cards: Query<(&WeaponCard, &mut BackgroundColor)>,
) {
    if state.phase != Phase::Hub {
        return;
    }
    if let Ok(mut t) = currency.single_mut() {
        t.0 = format!("💎 {}   🟠 {}   🟪 {}", battle.diamonds, battle.orbs, battle.cubes);
    }
    for (card, mut bg) in &mut cards {
        let owned = battle.owned_weapons.contains(&card.0);
        let affordable = battle.weapon_prices.get(&card.0).is_some_and(|c| battle.orbs >= *c);
        bg.0 = if owned {
            Color::srgb(0.08, 0.25, 0.14)
        } else if affordable {
            Color::srgb(0.14, 0.18, 0.30)
        } else {
            Color::srgb(0.10, 0.10, 0.15)
        };
    }
}

fn refresh_hud(
    state: Res<GameState>,
    battle: Res<Battle>,
    hud: Query<Entity, With<HudRoot>>,
    mut status: Query<&mut Text, With<StatusLabel>>,
    mut commands: Commands,
) {
    match state.phase {
        Phase::Battle => {
            if hud.is_empty() {
                commands
                    .spawn((
                        HudRoot,
                        Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                    ))
                    .with_children(|p| {
                        p.spawn((StatusLabel, label(&hud_text(&battle), 16.0)));
                    });
            } else if let Ok(mut t) = status.single_mut() {
                t.0 = hud_text(&battle);
            }
        }
        _ => {
            if let Ok(e) = hud.single() {
                commands.entity(e).despawn();
            }
        }
    }
}

fn hud_text(b: &Battle) -> String {
    format!(
        "HP {:.0}/{}   💎 {}   orcs {}   kills {}   leaks {}   {:?}",
        b.base_hp, b.base_hp_max, b.diamonds, b.orcs_alive, b.kills, b.leaks, b.outcome
    )
}

fn check_battle_end(
    battle: Res<Battle>,
    mut state: ResMut<GameState>,
    hub: Query<Entity, With<HubRoot>>,
    hud: Query<Entity, With<HudRoot>>,
    mut commands: Commands,
    content: Res<Content>,
) {
    if battle.outcome.is_some() && state.phase == Phase::Battle {
        state.phase = Phase::Hub;
        if let Ok(e) = hud.single() {
            commands.entity(e).despawn();
        }
        if hub.is_empty() {
            spawn_hub(commands, content, battle);
        }
    }
}

pub fn battle_started(state: Res<GameState>) -> bool {
    state.phase == Phase::Battle
}
