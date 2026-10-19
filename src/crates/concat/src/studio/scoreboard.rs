// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! The scoreboard clip, as the window edits it: placing one, recording its
//! points at the playhead, and the Scoreboard tab's every knob.
//!
//! Every edit replaces the clip's whole board through one `UpdateClip`, so
//! each is one undo step and the document never holds half of one. The
//! points are kept in the board's own time - see `ScoreEvent::at` - so the
//! playhead is turned into that time here, and back for the list.

use concat_project::model::{ScoreEvent, ScoreSide, ScoreboardLayout, ScoreboardStyle};

use super::*;

/// How long a board placed from the library runs when the timeline is
/// shorter than this past the playhead: long enough to see and to trim.
const SCOREBOARD_MIN_DURATION: f64 = 10.0;

impl Studio {
    /// A board from the Stickers page, placed over the picture from
    /// `start` to the end of the timeline, as a match's board is wanted.
    pub(super) fn add_scoreboard(
        &mut self,
        track_id: Option<String>,
        start: f64,
        layout: &str,
        name: &str,
    ) -> Option<String> {
        let layout = ScoreboardLayout::parse(layout).unwrap_or_default();
        let duration = (f64::from(self.duration()) - start).max(SCOREBOARD_MIN_DURATION);
        let style = ScoreboardStyle::preset(layout);
        let (offset_x, offset_y, scale) = self.corner_for(&style);
        self.apply(Command::AddScoreboardClip {
            track_id,
            above: true,
            start,
            style: Some(style),
            duration: Some(duration),
            name: name.to_owned(),
            offset_x: Some(offset_x),
            offset_y: Some(offset_y),
            scale: Some(scale),
        })
    }

    /// Where a new board goes: the top-left corner, where a broadcast keeps
    /// its score - a board in the middle of the picture covers the play -
    /// or along the top, shrunk to fit, on a frame too narrow for a corner.
    fn corner_for(&self, style: &ScoreboardStyle) -> (f64, f64, f64) {
        let (width, height) = self.output_size();
        let aspect = f64::from(width.max(1)) / f64::from(height.max(1));
        // About how wide the painter makes a board, as a fraction of the
        // frame's width: three and a half heights.
        let wide = 3.6 * style.size / aspect;
        let tall = style.size;
        let margin = 0.04;
        if wide <= 0.6 {
            (
                -(0.5 - margin - wide / 2.0),
                -(0.5 - margin - tall / 2.0),
                1.0,
            )
        } else {
            // The painter shrinks a board too wide for the frame to fit it;
            // the clip stays at its own scale, centred along the top.
            let fitted = tall * (0.96 / wide).min(1.0);
            (0.0, -(0.5 - margin - fitted / 2.0), 1.0)
        }
    }

    /// The selected scoreboard clip, when the selection is exactly one.
    fn selected_board(&self) -> Option<&Clip> {
        let id = self.sole_selection()?;
        self.clip(&id)
            .filter(|clip| clip.kind == model::ClipKind::Scoreboard)
    }

    /// The playhead in the selected board's own time, when it is over the
    /// board.
    fn board_time(clip: &Clip, playhead: f32) -> Option<f64> {
        let at = f64::from(playhead);
        (clip.start..clip.start + clip.duration)
            .contains(&at)
            .then(|| clip.source_start + (at - clip.start))
    }

    /// Changes the selected board with `change` and lands it as one edit.
    fn edit_board(&mut self, change: impl FnOnce(&mut ScoreboardStyle, &Clip)) {
        let Some(clip) = self.selected_board().cloned() else {
            return;
        };
        let mut board = clip.scoreboard.clone().unwrap_or_default();
        let before = board.clone();
        change(&mut board, &clip);
        if board == before {
            return;
        }
        self.apply(Command::UpdateClip {
            clip_id: clip.id.clone(),
            patch: ClipPatch {
                scoreboard: Some(board),
                ..ClipPatch::default()
            },
        });
    }

    pub fn scoreboard_text(&mut self, field: &str, value: &str) {
        if field == "font-family" {
            self.ensure_font(value);
        }
        let value = value.trim().to_owned();
        self.edit_board(|board, _| match field {
            "home-name" => board.home_name = value,
            "away-name" => board.away_name = value,
            "title" => board.title = value,
            "font-family" => board.font_family = value,
            _ => {}
        });
    }

    pub fn scoreboard_colour(&mut self, field: &str, value: slint::Color) {
        let hex = hex_rgba(value);
        self.edit_board(|board, _| {
            let slot = match field {
                "home-color" => &mut board.home_color,
                "away-color" => &mut board.away_color,
                "panel" => &mut board.panel,
                "ink" => &mut board.ink,
                "accent" => &mut board.accent,
                "accent-ink" => &mut board.accent_ink,
                "score" => &mut board.score,
                "serve-color" => &mut board.serve_color,
                _ => return,
            };
            *slot = hex;
        });
    }

    pub fn scoreboard_flag(&mut self, field: &str, on: bool) {
        self.edit_board(|board, _| match field {
            "visible" => board.visible = on,
            "show-icon" => board.show_icon = on,
            "show-games" => board.show_games = on,
            "show-serve" => board.show_serve = on,
            _ => {}
        });
    }

    pub fn scoreboard_number(&mut self, field: &str, value: f32) {
        let whole = value.round().max(0.0) as u32;
        self.edit_board(|board, _| match field {
            "size" => board.size = f64::from(value).clamp(0.03, 1.0),
            "game-points" => board.game_points = whole,
            "start-home" => board.start_home = whole,
            "start-away" => board.start_away = whole,
            "start-home-games" => board.start_home_games = whole,
            "start-away-games" => board.start_away_games = whole,
            "first-serve" => {
                board.first_serve = if whole == 1 {
                    ScoreSide::Away
                } else {
                    ScoreSide::Home
                }
            }
            _ => {}
        });
    }

    /// Another look for the selected board; the names and the match stay.
    pub fn scoreboard_layout(&mut self, row: i32) {
        let Some(layout) = ScoreboardLayout::ALL.get(row.max(0) as usize).copied() else {
            return;
        };
        self.edit_board(|board, _| {
            if board.layout != layout {
                *board = board.restyled(layout);
            }
        });
    }

    /// A point for one side - or one taken back - at the playhead.
    pub fn scoreboard_point(&mut self, home: bool, delta: i32) {
        let Some(clip) = self.selected_board() else {
            return;
        };
        let Some(at) = Self::board_time(clip, self.playhead) else {
            self.notify(&t("scoreboardPanel.outsideClip"), true);
            return;
        };
        self.edit_board(|board, _| {
            board.events.push(ScoreEvent {
                at,
                side: if home {
                    ScoreSide::Home
                } else {
                    ScoreSide::Away
                },
                delta: delta.signum(),
            });
            board.sort_events();
        });
    }

    pub fn scoreboard_event_remove(&mut self, row: i32) {
        self.edit_board(|board, _| {
            if (0..board.events.len() as i32).contains(&row) {
                board.events.remove(row as usize);
            }
        });
    }

    pub fn scoreboard_events_clear(&mut self) {
        self.edit_board(|board, _| board.events.clear());
    }

    /// The playhead to one recorded point: the frame its score shows on.
    pub fn scoreboard_event_seek(&mut self, row: i32) {
        let Some(clip) = self.selected_board() else {
            return;
        };
        let board = clip.scoreboard.clone().unwrap_or_default();
        let Some(event) = board.events.get(row.max(0) as usize) else {
            return;
        };
        let at = clip.start + (event.at - clip.source_start);
        self.seek(at.max(clip.start) as f32);
    }

    /// What the Scoreboard tab shows: the board, and its points.
    pub(super) fn scoreboard_data(&self) -> (ScoreboardData, Vec<ScoreEventData>) {
        let Some(clip) = self.selected_board() else {
            return (ScoreboardData::default(), Vec::new());
        };
        let board = clip.scoreboard.clone().unwrap_or_default();
        let now = Self::board_time(clip, self.playhead);
        // Outside the clip the panel shows the score at its nearer end.
        let shown_at = now.unwrap_or(if f64::from(self.playhead) < clip.start {
            clip.source_start
        } else {
            clip.source_start + clip.duration
        });
        let state = board.state_at(shown_at);
        let family = board.font_family.trim_matches('"').to_owned();
        let missing = !family.trim().is_empty()
            && !self
                .font_families()
                .iter()
                .any(|known| known.eq_ignore_ascii_case(&family));
        let layout = ScoreboardLayout::ALL
            .iter()
            .position(|layout| *layout == board.layout)
            .unwrap_or(0);
        let data = ScoreboardData {
            present: true,
            id: clip.id.as_str().into(),
            layout: layout as i32,
            visible: board.visible,
            home_name: board.home_name.as_str().into(),
            away_name: board.away_name.as_str().into(),
            title: board.title.as_str().into(),
            home_color: colour_of(&board.home_color),
            away_color: colour_of(&board.away_color),
            panel: colour_of(&board.panel),
            ink: colour_of(&board.ink),
            accent: colour_of(&board.accent),
            accent_ink: colour_of(&board.accent_ink),
            score: colour_of(&board.score),
            serve_color: colour_of(&board.serve_color),
            font_family: family.as_str().into(),
            font_missing: missing,
            size: board.size as f32,
            show_icon: board.show_icon,
            show_games: board.show_games,
            show_serve: board.show_serve,
            has_icon: matches!(
                board.layout,
                ScoreboardLayout::Classic | ScoreboardLayout::Tournament
            ),
            has_header: matches!(
                board.layout,
                ScoreboardLayout::Broadcast | ScoreboardLayout::Tournament
            ),
            game_points: board.game_points as i32,
            start_home: board.start_home as i32,
            start_away: board.start_away as i32,
            start_home_games: board.start_home_games as i32,
            start_away_games: board.start_away_games as i32,
            first_serve: (board.first_serve == ScoreSide::Away) as i32,
            home_points: state.home_points as i32,
            away_points: state.away_points as i32,
            home_games: state.home_games as i32,
            away_games: state.away_games as i32,
            in_clip: now.is_some(),
        };
        // The last point at or before the playhead is the one the board
        // is showing.
        let current = board.events.iter().rposition(|event| event.at <= shown_at);
        let events = board
            .events
            .iter()
            .enumerate()
            .map(|(row, event)| {
                let after = board.state_at(event.at);
                let timeline = clip.start + (event.at - clip.source_start);
                ScoreEventData {
                    time: clock(timeline).into(),
                    side: match event.side {
                        ScoreSide::Home => "A",
                        ScoreSide::Away => "B",
                    }
                    .into(),
                    plus: event.delta > 0,
                    score: format!("{} - {}", after.home_points, after.away_points).into(),
                    current: current == Some(row),
                }
            })
            .collect();
        (data, events)
    }
}

/// A picture of each of the ten looks, for the library's cards: a board
/// mid-match, cut out of a 720p frame with a little room for its shadow,
/// as straight RGBA. Slow enough - ten boards, fonts loaded first - to be
/// made on a worker.
pub fn scoreboard_previews() -> Vec<(String, slint::SharedPixelBuffer<slint::Rgba8Pixel>)> {
    use concat_project::model::ScoreState;
    let fonts = concat_text::Fonts::new();
    let state = ScoreState {
        home_points: 21,
        away_points: 18,
        home_games: 1,
        away_games: 0,
        serving: Some(ScoreSide::Home),
    };
    let (width, height) = (1280_u32, 720_u32);
    ScoreboardLayout::ALL
        .iter()
        .filter_map(|layout| {
            let style = ScoreboardStyle::preset(*layout);
            let paint = concat_host::titles::board_of(&style, state);
            let frame = concat_text::render_scoreboard_frame(&fonts, &paint, width, height).ok()?;
            let margin = 24;
            let w = (frame.block_width + 2 * margin).min(width);
            let h = (frame.block_height + 2 * margin).min(height);
            let left = (width - w) / 2;
            let top = (height - h) / 2;
            let mut pixels = Vec::with_capacity((w * h * 4) as usize);
            for row in top..top + h {
                let at = ((row * width + left) * 4) as usize;
                pixels.extend_from_slice(&frame.rgba[at..at + (w * 4) as usize]);
            }
            let buffer =
                slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&pixels, w, h);
            Some((layout.id().to_owned(), buffer))
        })
        .collect()
}

/// A moment as "m:ss.s", which is what a list of rallies wants: finer than
/// a second, and no hours.
fn clock(seconds: f64) -> String {
    let tenths = (seconds.max(0.0) * 10.0).round() as u64;
    format!("{}:{:02}.{}", tenths / 600, (tenths / 10) % 60, tenths % 10)
}

#[cfg(test)]
mod tests {
    use super::clock;

    #[test]
    fn a_moment_reads_as_minutes_seconds_and_tenths() {
        assert_eq!(clock(0.0), "0:00.0");
        assert_eq!(clock(72.46), "1:12.5");
        assert_eq!(clock(-3.0), "0:00.0");
    }
}
