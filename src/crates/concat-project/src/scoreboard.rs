// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! A scoreboard: two sides, their names and colours, and the points they
//! win as the match plays.
//!
//! A scoreboard clip is drawn over the picture like a title, but what it
//! shows changes along its length: every point is an event at a moment of
//! the board's own time, and the score at any moment is the events up to
//! it, replayed.
//! The style is the look - one of ten layouts and the colours the user has
//! made their own - and the events are the match. Both live in the one
//! struct so a copy, a split or an undo carries the match with its board.
//!
//! The replay knows badminton's rules when asked to: a game goes to the
//! side that reaches [`ScoreboardStyle::game_points`] two clear, or first
//! to the cap, and the next rally starts the next game from love. The side
//! that won the last rally serves, which is what the shuttle beside a name
//! says.

use serde::{Deserialize, Serialize};

/// One side of the board: the top row, or the bottom.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScoreSide {
    /// The top row, or the left of a one-row board.
    #[default]
    Home,
    /// The bottom row, or the right.
    Away,
}

/// One change of score: a point won, or one taken back, at a moment of the
/// clip.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreEvent {
    /// Seconds of the board's own time: the clip's in-point plus the time
    /// into the clip. Its own time and not the clip's, so a head trimmed
    /// off or a split leaves every point on the frame it was scored on.
    pub at: f64,
    /// Whose point.
    pub side: ScoreSide,
    /// +1 for a point won; -1 takes that side's last point back.
    pub delta: i32,
}

/// The ten looks a scoreboard comes in. The names are written into project
/// files, so they are stable for ever.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScoreboardLayout {
    /// Navy, a trophy, team-colour blocks and blue game boxes.
    #[default]
    Classic,
    /// A television strip: a red header over a charcoal board.
    Broadcast,
    /// A dark glass pill, names and points and nothing else.
    Minimal,
    /// Black with a glowing outline.
    Neon,
    /// Court green with a white line round it.
    Court,
    /// Deep teal and sea-glass.
    Ocean,
    /// Plum and orange.
    Sunrise,
    /// One row: both names either side of the score.
    Slate,
    /// Black on white, like a printed scoresheet.
    Ink,
    /// A gold header and trophy, for a final.
    Tournament,
}

impl ScoreboardLayout {
    /// Every layout, in the order the library shows them.
    pub const ALL: [ScoreboardLayout; 10] = [
        ScoreboardLayout::Classic,
        ScoreboardLayout::Broadcast,
        ScoreboardLayout::Minimal,
        ScoreboardLayout::Neon,
        ScoreboardLayout::Court,
        ScoreboardLayout::Ocean,
        ScoreboardLayout::Sunrise,
        ScoreboardLayout::Slate,
        ScoreboardLayout::Ink,
        ScoreboardLayout::Tournament,
    ];

    /// The name a document writes.
    pub fn id(self) -> &'static str {
        match self {
            ScoreboardLayout::Classic => "classic",
            ScoreboardLayout::Broadcast => "broadcast",
            ScoreboardLayout::Minimal => "minimal",
            ScoreboardLayout::Neon => "neon",
            ScoreboardLayout::Court => "court",
            ScoreboardLayout::Ocean => "ocean",
            ScoreboardLayout::Sunrise => "sunrise",
            ScoreboardLayout::Slate => "slate",
            ScoreboardLayout::Ink => "ink",
            ScoreboardLayout::Tournament => "tournament",
        }
    }

    /// The layout `id` names, if this build knows it.
    pub fn parse(id: &str) -> Option<ScoreboardLayout> {
        ScoreboardLayout::ALL
            .into_iter()
            .find(|layout| layout.id() == id)
    }
}

/// A scoreboard's look and its match. Sizes are fractions of the frame's
/// height, as a title's are. A style arriving in a command needs only the
/// fields it changes.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScoreboardStyle {
    /// Which of the ten looks.
    #[serde(deserialize_with = "layout")]
    pub layout: ScoreboardLayout,
    /// Shown at all. Off keeps the board and its match on the timeline
    /// without drawing it.
    pub visible: bool,
    /// The top row's name.
    pub home_name: String,
    /// The bottom row's name.
    pub away_name: String,
    /// The top row's colour, `#rrggbb`.
    pub home_color: String,
    /// The bottom row's colour.
    pub away_color: String,
    /// The board itself, `#rrggbb[aa]`.
    pub panel: String,
    /// The names.
    pub ink: String,
    /// The game boxes, a header, a glow: the layout's second colour.
    pub accent: String,
    /// What is written on the accent.
    pub accent_ink: String,
    /// The points.
    pub score: String,
    /// The shuttle beside the side serving.
    pub serve_color: String,
    /// CSS-style family name, as a title's.
    pub font_family: String,
    /// CSS-scale weight, 100..=900.
    pub font_weight: f64,
    /// The board's height as a fraction of frame height.
    pub size: f64,
    /// A header line - the event, the round - for the layouts that have one.
    pub title: String,
    /// The trophy at the board's left, for the layouts that draw one.
    pub show_icon: bool,
    /// The games won, beside the points.
    pub show_games: bool,
    /// The shuttle beside the side serving.
    pub show_serve: bool,
    /// Points that win a game, two clear, with badminton's cap; zero
    /// counts points for ever and leaves the games as they started.
    pub game_points: u32,
    /// The score the clip opens on, for a clip that starts mid-match.
    pub start_home: u32,
    /// See `start_home`.
    pub start_away: u32,
    /// The games already won when the clip opens.
    pub start_home_games: u32,
    /// See `start_home_games`.
    pub start_away_games: u32,
    /// Who serves before any point is recorded, so the shuttle is on the
    /// board from the first frame. After that the side that won the last
    /// rally serves.
    #[serde(deserialize_with = "side")]
    pub first_serve: ScoreSide,
    /// The match: every point, in the order it was won. Kept sorted by
    /// time.
    pub events: Vec<ScoreEvent>,
}

/// What the board shows at one moment.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreState {
    /// The top row's points in the game being played.
    pub home_points: u32,
    /// The bottom row's.
    pub away_points: u32,
    /// The top row's games won.
    pub home_games: u32,
    /// The bottom row's.
    pub away_games: u32,
    /// Who won the last rally, and so serves; none before the first.
    pub serving: Option<ScoreSide>,
}

impl Default for ScoreboardStyle {
    fn default() -> ScoreboardStyle {
        ScoreboardStyle::preset(ScoreboardLayout::Classic)
    }
}

impl ScoreboardStyle {
    /// A fresh board in `layout`'s own colours and face, with placeholder
    /// names and no points yet.
    pub fn preset(layout: ScoreboardLayout) -> ScoreboardStyle {
        let base = ScoreboardStyle {
            layout,
            visible: true,
            home_name: "Flamingos".to_owned(),
            away_name: "Parakeets".to_owned(),
            home_color: "#f2386f".to_owned(),
            away_color: "#ffd31a".to_owned(),
            panel: "#0b1533".to_owned(),
            ink: "#ffffff".to_owned(),
            accent: "#5b8def".to_owned(),
            accent_ink: "#0b1533".to_owned(),
            score: "#ffffff".to_owned(),
            serve_color: "#d68cf5".to_owned(),
            font_family: "Bebas Neue".to_owned(),
            font_weight: 400.0,
            size: 0.2,
            title: String::new(),
            show_icon: true,
            show_games: true,
            show_serve: true,
            game_points: 21,
            start_home: 0,
            start_away: 0,
            start_home_games: 0,
            start_away_games: 0,
            first_serve: ScoreSide::Home,
            events: Vec::new(),
        };
        let colours = |panel: &str,
                       ink: &str,
                       accent: &str,
                       accent_ink: &str,
                       score: &str,
                       home: &str,
                       away: &str,
                       serve: &str| ScoreboardStyle {
            panel: panel.to_owned(),
            ink: ink.to_owned(),
            accent: accent.to_owned(),
            accent_ink: accent_ink.to_owned(),
            score: score.to_owned(),
            home_color: home.to_owned(),
            away_color: away.to_owned(),
            serve_color: serve.to_owned(),
            ..base.clone()
        };
        match layout {
            ScoreboardLayout::Classic => base,
            ScoreboardLayout::Broadcast => ScoreboardStyle {
                font_family: "Anton".to_owned(),
                title: "Badminton · Final".to_owned(),
                show_icon: false,
                ..colours(
                    "#17191f", "#ffffff", "#e5243b", "#ffffff", "#ffffff", "#e5243b", "#2d7ff9",
                    "#ffd31a",
                )
            },
            ScoreboardLayout::Minimal => ScoreboardStyle {
                font_family: "Inter".to_owned(),
                font_weight: 700.0,
                show_icon: false,
                ..colours(
                    "#000000b3",
                    "#ffffff",
                    "#ffffff33",
                    "#ffffff",
                    "#ffffff",
                    "#ff5a5f",
                    "#3ddc97",
                    "#ffffff",
                )
            },
            ScoreboardLayout::Neon => ScoreboardStyle {
                font_family: "Space Grotesk".to_owned(),
                font_weight: 700.0,
                show_icon: false,
                ..colours(
                    "#05060ae6",
                    "#ffffff",
                    "#00e5ff",
                    "#05060a",
                    "#ff3df2",
                    "#ff3df2",
                    "#00e5ff",
                    "#faff00",
                )
            },
            ScoreboardLayout::Court => ScoreboardStyle {
                show_icon: false,
                ..colours(
                    "#1d6b3c", "#ffffff", "#ffffff", "#1d6b3c", "#ffffff", "#ffb703", "#8ecae6",
                    "#ffffff",
                )
            },
            ScoreboardLayout::Ocean => ScoreboardStyle {
                font_family: "Montserrat".to_owned(),
                font_weight: 800.0,
                show_icon: false,
                ..colours(
                    "#04364a", "#ffffff", "#64ccc5", "#04364a", "#dafffb", "#ff6b6b", "#ffd93d",
                    "#dafffb",
                )
            },
            ScoreboardLayout::Sunrise => ScoreboardStyle {
                font_family: "Archivo Black".to_owned(),
                show_icon: false,
                ..colours(
                    "#2d1b3d", "#ffffff", "#ff7a45", "#2d1b3d", "#ffffff", "#ff4f8b", "#ffc93c",
                    "#ffc93c",
                )
            },
            ScoreboardLayout::Slate => ScoreboardStyle {
                show_icon: false,
                ..colours(
                    "#2f3640", "#f5f6fa", "#f5f6fa", "#2f3640", "#2f3640", "#e84118", "#00a8ff",
                    "#fbc531",
                )
            },
            ScoreboardLayout::Ink => ScoreboardStyle {
                font_family: "Montserrat".to_owned(),
                font_weight: 800.0,
                show_icon: false,
                ..colours(
                    "#ffffff", "#111111", "#111111", "#ffffff", "#111111", "#e63946", "#1d3557",
                    "#e63946",
                )
            },
            ScoreboardLayout::Tournament => ScoreboardStyle {
                font_family: "Anton".to_owned(),
                title: "Championship Final".to_owned(),
                ..colours(
                    "#111827", "#ffffff", "#d4af37", "#111827", "#ffffff", "#d62828", "#1d4ed8",
                    "#d4af37",
                )
            },
        }
    }

    /// This board in another layout: the new layout's colours and face,
    /// with the names, the switches, the rules and the match kept.
    pub fn restyled(&self, layout: ScoreboardLayout) -> ScoreboardStyle {
        let look = ScoreboardStyle::preset(layout);
        ScoreboardStyle {
            visible: self.visible,
            home_name: self.home_name.clone(),
            away_name: self.away_name.clone(),
            size: self.size,
            title: if self.title.is_empty() {
                look.title.clone()
            } else {
                self.title.clone()
            },
            show_games: self.show_games,
            show_serve: self.show_serve,
            game_points: self.game_points,
            start_home: self.start_home,
            start_away: self.start_away,
            start_home_games: self.start_home_games,
            start_away_games: self.start_away_games,
            first_serve: self.first_serve,
            events: self.events.clone(),
            ..look
        }
    }

    /// Every number back in its range and the events in time order; see
    /// `TextStyle::tidy`.
    pub fn tidy(mut self) -> ScoreboardStyle {
        fn finite(value: f64, fallback: f64) -> f64 {
            if value.is_finite() { value } else { fallback }
        }
        self.size = finite(self.size, 0.2).clamp(0.03, 1.0);
        self.font_weight = finite(self.font_weight, 400.0).clamp(100.0, 900.0);
        self.game_points = self.game_points.min(99);
        self.start_home = self.start_home.min(999);
        self.start_away = self.start_away.min(999);
        self.start_home_games = self.start_home_games.min(99);
        self.start_away_games = self.start_away_games.min(99);
        self.events
            .retain(|event| event.at.is_finite() && event.delta != 0);
        for event in &mut self.events {
            event.at = event.at.max(0.0);
            event.delta = event.delta.signum();
        }
        self.sort_events();
        self
    }

    /// Puts the events in time order, keeping the order of those at the
    /// same moment.
    pub fn sort_events(&mut self) {
        self.events.sort_by(|a, b| a.at.total_cmp(&b.at));
    }

    /// The score `at` seconds into the clip: every event up to and
    /// including that moment, replayed.
    pub fn state_at(&self, at: f64) -> ScoreState {
        // The points that stand: a +1 adds one, a -1 takes back that
        // side's most recent - which may be across a game boundary, and
        // the replay then puts the game back too.
        let mut won: Vec<ScoreSide> = Vec::new();
        for event in self.events.iter().filter(|event| event.at <= at) {
            if event.delta > 0 {
                won.push(event.side);
            } else if let Some(last) = won.iter().rposition(|side| *side == event.side) {
                won.remove(last);
            }
        }
        let mut state = ScoreState {
            home_points: self.start_home,
            away_points: self.start_away,
            home_games: self.start_home_games,
            away_games: self.start_away_games,
            serving: Some(self.first_serve),
        };
        // A game won stays on the board until the next rally starts the
        // next one, so the winning score is seen.
        let mut game_over = false;
        for side in won {
            if game_over {
                state.home_points = 0;
                state.away_points = 0;
                game_over = false;
            }
            let (mine, theirs) = match side {
                ScoreSide::Home => (&mut state.home_points, state.away_points),
                ScoreSide::Away => (&mut state.away_points, state.home_points),
            };
            *mine += 1;
            let mine = *mine;
            state.serving = Some(side);
            if self.game_points > 0 {
                let target = self.game_points;
                // Badminton's cap: 30 for a game to 21, 21 for one to 15.
                let cap = target * 10 / 7;
                if (mine >= target && mine >= theirs + 2) || mine >= cap.max(target) {
                    match side {
                        ScoreSide::Home => state.home_games += 1,
                        ScoreSide::Away => state.away_games += 1,
                    }
                    game_over = true;
                }
            }
        }
        state
    }

    /// The board between `from` and `to` in its own time, as spans of
    /// unchanging score: `(from, to, state)`, in order, covering the whole
    /// stretch, no two neighbours alike.
    pub fn spans(&self, from: f64, to: f64) -> Vec<(f64, f64, ScoreState)> {
        let to = to.max(from);
        let mut cuts: Vec<f64> = self
            .events
            .iter()
            .map(|event| event.at)
            .filter(|at| *at > from && *at < to)
            .collect();
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        let mut out: Vec<(f64, f64, ScoreState)> = Vec::with_capacity(cuts.len() + 1);
        let mut start = from;
        for end in cuts.into_iter().chain(std::iter::once(to)) {
            let state = self.state_at(start);
            match out.last_mut() {
                Some(last) if last.2 == state => last.1 = end,
                _ => out.push((start, end, state)),
            }
            start = end;
        }
        out
    }
}

/// A side by name, home for one this build does not know.
fn side<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<ScoreSide, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

/// A layout by name, the classic one for a name this build does not know.
fn layout<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<ScoreboardLayout, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(at: f64, side: ScoreSide) -> ScoreEvent {
        ScoreEvent { at, side, delta: 1 }
    }

    #[test]
    fn points_add_up_and_the_last_to_score_serves() {
        let mut board = ScoreboardStyle::default();
        board.events = vec![
            point(1.0, ScoreSide::Home),
            point(2.0, ScoreSide::Home),
            point(3.0, ScoreSide::Away),
        ];
        // Before the first point the side named to serve first serves.
        assert_eq!(
            board.state_at(0.5),
            ScoreState {
                serving: Some(ScoreSide::Home),
                ..ScoreState::default()
            }
        );
        board.first_serve = ScoreSide::Away;
        assert_eq!(board.state_at(0.5).serving, Some(ScoreSide::Away));
        assert_eq!(
            board.state_at(1.0).serving,
            Some(ScoreSide::Home),
            "then the scorer"
        );
        let state = board.state_at(3.0);
        assert_eq!((state.home_points, state.away_points), (2, 1));
        assert_eq!(state.serving, Some(ScoreSide::Away));
    }

    #[test]
    fn a_minus_takes_back_that_sides_last_point() {
        let mut board = ScoreboardStyle::default();
        board.events = vec![
            point(1.0, ScoreSide::Home),
            point(2.0, ScoreSide::Away),
            ScoreEvent {
                at: 3.0,
                side: ScoreSide::Home,
                delta: -1,
            },
            // Nothing left to take back: a no-op, not a negative score.
            ScoreEvent {
                at: 4.0,
                side: ScoreSide::Home,
                delta: -1,
            },
        ];
        let state = board.state_at(10.0);
        assert_eq!((state.home_points, state.away_points), (0, 1));
    }

    #[test]
    fn a_game_goes_to_twenty_one_two_clear_and_the_next_rally_starts_the_next() {
        let mut board = ScoreboardStyle {
            start_home: 20,
            start_away: 19,
            ..ScoreboardStyle::default()
        };
        board.events = vec![point(1.0, ScoreSide::Home), point(2.0, ScoreSide::Away)];
        let won = board.state_at(1.0);
        assert_eq!(
            (won.home_points, won.away_points, won.home_games),
            (21, 19, 1)
        );
        let next = board.state_at(2.0);
        assert_eq!((next.home_points, next.away_points), (0, 1));
        assert_eq!((next.home_games, next.away_games), (1, 0));

        // Deuce runs on to the cap.
        let capped = ScoreboardStyle {
            start_home: 29,
            start_away: 29,
            events: vec![point(1.0, ScoreSide::Away)],
            ..ScoreboardStyle::default()
        };
        let state = capped.state_at(1.0);
        assert_eq!((state.away_points, state.away_games), (30, 1));

        // Taking the winning point back takes the game back.
        board.events = vec![
            point(1.0, ScoreSide::Home),
            ScoreEvent {
                at: 5.0,
                side: ScoreSide::Home,
                delta: -1,
            },
        ];
        let state = board.state_at(5.0);
        assert_eq!(
            (state.home_points, state.away_points, state.home_games),
            (20, 19, 0)
        );
    }

    #[test]
    fn spans_cover_the_clip_and_merge_what_does_not_change() {
        let mut board = ScoreboardStyle::default();
        board.events = vec![
            point(1.0, ScoreSide::Home),
            ScoreEvent {
                at: 2.0,
                side: ScoreSide::Away,
                delta: -1,
            },
            point(3.0, ScoreSide::Away),
            point(9.0, ScoreSide::Away),
        ];
        let spans = board.spans(0.0, 5.0);
        assert_eq!(spans.len(), 3);
        assert_eq!((spans[0].0, spans[0].1), (0.0, 1.0));
        assert_eq!(
            (spans[1].0, spans[1].1),
            (1.0, 3.0),
            "the no-op merged away"
        );
        assert_eq!((spans[2].0, spans[2].1), (3.0, 5.0));
        assert_eq!(spans[2].2.away_points, 1);
    }

    #[test]
    fn a_new_layout_keeps_the_match() {
        let mut board = ScoreboardStyle::default();
        board.home_name = "Lin".into();
        board.events = vec![point(1.0, ScoreSide::Home)];
        let neon = board.restyled(ScoreboardLayout::Neon);
        assert_eq!(neon.layout, ScoreboardLayout::Neon);
        assert_eq!(neon.home_name, "Lin");
        assert_eq!(neon.events, board.events);
        assert_eq!(
            neon.panel,
            ScoreboardStyle::preset(ScoreboardLayout::Neon).panel
        );
    }
}
