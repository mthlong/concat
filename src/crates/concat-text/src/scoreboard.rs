// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! Scoreboards as pixels.
//!
//! A scoreboard clip is two sides and a score; the compositor wants the same
//! frame-sized transparent canvas a title gets, for the same reasons (see the
//! module docs). The board is centred on the canvas, so the clip's offset and
//! scale mean what they mean for a title, and `Rendered::block_*` is the
//! board's own box, for a monitor to outline.
//!
//! One painter draws all ten layouts: each is a [`Spec`] - which parts it has
//! and how they are dressed - over the same two arrangements, two rows
//! stacked or one row with the score in the middle. The board's width is set
//! by the names and by the widest score it could show, never by the score it
//! does show, so a board does not twitch when 9 becomes 10.

use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Rect,
    Stroke, Transform,
};
use unicode_bidi::ParagraphBidiInfo;

use crate::{
    Align, Block, Error, Fonts, Paragraph, Rendered, RenderedFrame, Setter, TitleStyle, blur,
    colour, push_rounded_rect, rgba, straight_rgba,
};

/// The looks a scoreboard comes in. Mirrors the document's own list; this
/// crate does not depend on the document.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Board {
    /// Navy, a trophy, team-colour blocks and blue game boxes.
    Classic,
    /// A header strip over a charcoal board.
    Broadcast,
    /// A dark glass pill.
    Minimal,
    /// A glowing outline.
    Neon,
    /// Court green with a white line round it.
    Court,
    /// Teal and sea-glass.
    Ocean,
    /// Plum and orange.
    Sunrise,
    /// One row, the score in the middle.
    Slate,
    /// Black on white.
    Ink,
    /// A header and a trophy, for a final.
    Tournament,
}

/// Everything about a scoreboard at one moment: its look, in the document's
/// terms, and the score it shows.
#[derive(Clone, PartialEq, Debug)]
pub struct Scoreboard {
    /// Which look.
    pub board: Board,
    /// The board's height as a fraction of frame height.
    pub size: f64,
    /// CSS-style family name.
    pub font_family: String,
    /// CSS-scale weight.
    pub font_weight: f64,
    /// The header's words, for a look that has a header; empty for none.
    pub title: String,
    /// The top row's name, or the left side's.
    pub home_name: String,
    /// The bottom row's, or the right side's.
    pub away_name: String,
    /// Colours, `#rrggbb[aa]`: each side's own,
    pub home_color: String,
    /// see `home_color`,
    pub away_color: String,
    /// the board,
    pub panel: String,
    /// the names,
    pub ink: String,
    /// the look's second colour - game boxes, header, glow -
    pub accent: String,
    /// what is written on it,
    pub accent_ink: String,
    /// the points,
    pub score: String,
    /// and the shuttle beside whoever serves.
    pub serve_color: String,
    /// The trophy, for a look that has room for one.
    pub show_icon: bool,
    /// The games won.
    pub show_games: bool,
    /// The shuttle.
    pub show_serve: bool,
    /// The score.
    pub home_points: u32,
    /// See `home_points`.
    pub away_points: u32,
    /// The games.
    pub home_games: u32,
    /// See `home_games`.
    pub away_games: u32,
    /// Who serves: `Some(true)` the home side, `Some(false)` the away
    /// side, `None` nobody yet.
    pub serving: Option<bool>,
}

/// Paints `board` centred on a `width` × `height` transparent canvas and
/// returns it PNG-encoded with the board's box. The counterpart of
/// [`crate::render`] for a scoreboard.
pub fn render_scoreboard(
    fonts: &Fonts,
    board: &Scoreboard,
    width: u32,
    height: u32,
) -> Result<Rendered, Error> {
    let (canvas, block) = paint_board(fonts, board, width, height)?;
    let png = canvas
        .encode_png()
        .map_err(|error| Error::Encode(error.to_string()))?;
    Ok(Rendered {
        png,
        width,
        height,
        block_width: block.0,
        block_height: block.1,
        block_dx: block.2,
        block_dy: block.3,
        words: Vec::new(),
    })
}

/// [`render_scoreboard`], but the pixels rather than a PNG of them.
pub fn render_scoreboard_frame(
    fonts: &Fonts,
    board: &Scoreboard,
    width: u32,
    height: u32,
) -> Result<RenderedFrame, Error> {
    let (canvas, block) = paint_board(fonts, board, width, height)?;
    Ok(RenderedFrame {
        rgba: straight_rgba(&canvas),
        width,
        height,
        block_width: block.0,
        block_height: block.1,
        block_dx: block.2,
        block_dy: block.3,
        words: Vec::new(),
    })
}

/// How a side's colour is shown beside its name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bar {
    /// A solid block the row's height, the two rows' meeting.
    Block,
    /// A slim rounded bar.
    Stripe,
    /// A dot.
    Dot,
}

/// Which parts a look has and how they are dressed.
struct Spec {
    /// Corner radius as a fraction of the board's height.
    radius: f32,
    /// A header strip when the title has words.
    header: bool,
    bar: Bar,
    /// The games on a box of the accent colour, else as dim figures.
    games_box: bool,
    /// The points on a box of the accent colour.
    points_box: bool,
    /// A glow of the accent round the board.
    glow: bool,
    /// A line inset round the board, in the ink.
    court_line: bool,
    /// A hairline between the rows.
    divider: bool,
    /// One row rather than two.
    inline: bool,
    /// The trophy is drawn when asked for.
    icon: bool,
    /// The trophy in the accent rather than the ink.
    icon_accent: bool,
}

impl Board {
    fn spec(self) -> Spec {
        let base = Spec {
            radius: 0.06,
            header: false,
            bar: Bar::Block,
            games_box: true,
            points_box: false,
            glow: false,
            court_line: false,
            divider: false,
            inline: false,
            icon: false,
            icon_accent: false,
        };
        match self {
            Board::Classic => Spec { icon: true, ..base },
            Board::Broadcast => Spec {
                radius: 0.03,
                header: true,
                bar: Bar::Stripe,
                games_box: false,
                points_box: true,
                divider: true,
                ..base
            },
            Board::Minimal => Spec {
                radius: 0.25,
                bar: Bar::Dot,
                games_box: false,
                divider: true,
                ..base
            },
            Board::Neon => Spec {
                radius: 0.12,
                bar: Bar::Stripe,
                games_box: false,
                glow: true,
                ..base
            },
            Board::Court => Spec {
                radius: 0.04,
                bar: Bar::Stripe,
                court_line: true,
                ..base
            },
            Board::Ocean => Spec {
                radius: 0.1,
                ..base
            },
            Board::Sunrise => Spec {
                radius: 0.14,
                bar: Bar::Stripe,
                games_box: false,
                points_box: true,
                ..base
            },
            Board::Slate => Spec {
                radius: 0.12,
                bar: Bar::Stripe,
                inline: true,
                ..base
            },
            Board::Ink => Spec {
                radius: 0.03,
                bar: Bar::Stripe,
                divider: true,
                ..base
            },
            Board::Tournament => Spec {
                header: true,
                icon: true,
                icon_accent: true,
                ..base
            },
        }
    }
}

/// One run of words, shaped and outlined with its pen at the origin on the
/// baseline: its path, its advance and its capital height, all in pixels.
struct Label {
    path: Option<Path>,
    width: f32,
    cap: f32,
}

/// The face's capital height per em, for sizing words by the height of
/// their capitals - what reads as "the same size" across faces.
fn cap_ratio(face: &rustybuzz::Face<'_>) -> f32 {
    let upem = face.units_per_em() as f32;
    face.capital_height()
        .filter(|cap| *cap > 0)
        .map_or(0.7, |cap| cap as f32 / upem)
}

/// Words set in `family` at `weight`, sized so their capitals are `cap`
/// pixels tall.
fn label(fonts: &Fonts, text: &str, family: &str, weight: f64, cap: f32) -> Result<Label, Error> {
    let text = text.lines().next().unwrap_or("").trim();
    let style = TitleStyle {
        content: text.to_owned(),
        font_family: family.to_owned(),
        font_size: 0.0,
        font_weight: weight,
        italic: false,
        color: "#ffffff".to_owned(),
        align: Align::Left,
        stroke_width: 0.0,
        stroke_color: String::new(),
        shadow: false,
        background: String::new(),
        background_radius: 0.0,
        background_padding_x: 0.0,
        background_padding_y: 0.0,
        line_height: 1.0,
        tracking: 0.0,
        max_width: 0.0,
        max_height: 0.0,
    };
    let casting = fonts.cast(&style)?;
    let own = {
        let (data, index) = &casting.faces[0];
        rustybuzz::Face::from_slice((**data).as_ref(), *index).ok_or(Error::BadFont)?
    };
    let em = cap / cap_ratio(&own);
    let Some(face_of) = casting.paragraphs.first() else {
        return Ok(Label {
            path: None,
            width: 0.0,
            cap,
        });
    };
    let faces: Vec<rustybuzz::Face<'_>> = casting
        .faces
        .iter()
        .map(|(data, index)| {
            rustybuzz::Face::from_slice((**data).as_ref(), *index).unwrap_or_else(|| own.clone())
        })
        .collect();
    let setter = Setter {
        faces: &faces,
        em,
        tracking: 0.0,
        foreground: rgba(Color::WHITE),
    };
    let para = Paragraph {
        text,
        bidi: ParagraphBidiInfo::new(text, None),
        face_of,
    };
    let line = setter.set(&para, 0..text.len(), true);
    Ok(Label {
        path: line.path,
        width: line.width,
        cap,
    })
}

/// Where a label sits in its column.
#[derive(Clone, Copy)]
enum Justify {
    Start,
    Middle,
}

fn solid(colour: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(colour);
    paint.anti_alias = true;
    paint
}

/// `colour` at `alpha` times its own opacity.
fn faded(colour: Color, alpha: f32) -> Color {
    let mut out = colour;
    out.set_alpha((colour.alpha() * alpha).clamp(0.0, 1.0));
    out
}

/// Draws `label` in the column `x .. x + w`, its capitals centred on `cy`,
/// squeezed to fit the column when it is wider.
fn draw_label(
    canvas: &mut Pixmap,
    label: &Label,
    (x, w): (f32, f32),
    cy: f32,
    justify: Justify,
    colour: Color,
) {
    let Some(path) = &label.path else { return };
    let scale = if w > 0.0 && label.width > w {
        w / label.width
    } else {
        1.0
    };
    let shown = label.width * scale;
    let left = match justify {
        Justify::Start => x,
        Justify::Middle => x + (w - shown) / 2.0,
    };
    let baseline = cy + label.cap * scale / 2.0;
    canvas.fill_path(
        path,
        &solid(colour),
        FillRule::Winding,
        Transform::from_row(scale, 0.0, 0.0, scale, left, baseline),
        None,
    );
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Option<Rect> {
    Rect::from_xywh(x, y, w.max(0.01), h.max(0.01))
}

fn rounded(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Option<Path> {
    let mut builder = PathBuilder::new();
    push_rounded_rect(&mut builder, rect(x, y, w, h)?, radius);
    builder.finish()
}

/// A rectangle rounded only along its top or only along its bottom.
fn half_rounded(x: f32, y: f32, w: f32, h: f32, radius: f32, top: bool) -> Option<Path> {
    const KAPPA: f32 = 0.552_284_8;
    let r = radius.min(w / 2.0).min(h).max(0.0);
    let k = r * (1.0 - KAPPA);
    let (l, t, rt, b) = (x, y, x + w, y + h);
    let mut p = PathBuilder::new();
    if top {
        p.move_to(l, b);
        p.line_to(l, t + r);
        p.cubic_to(l, t + k, l + k, t, l + r, t);
        p.line_to(rt - r, t);
        p.cubic_to(rt - k, t, rt, t + k, rt, t + r);
        p.line_to(rt, b);
    } else {
        p.move_to(l, t);
        p.line_to(rt, t);
        p.line_to(rt, b - r);
        p.cubic_to(rt, b - k, rt - k, b, rt - r, b);
        p.line_to(l + r, b);
        p.cubic_to(l + k, b, l, b - k, l, b - r);
    }
    p.close();
    p.finish()
}

/// A trophy on lucide's 24 grid: the cup and its handles, stem and foot as
/// one stroked path, and the star in the cup as a filled one.
fn trophy() -> (Option<Path>, Option<Path>) {
    let mut cup = PathBuilder::new();
    cup.move_to(6.0, 2.5);
    cup.line_to(18.0, 2.5);
    cup.line_to(18.0, 9.0);
    cup.cubic_to(18.0, 12.3, 15.3, 15.0, 12.0, 15.0);
    cup.cubic_to(8.7, 15.0, 6.0, 12.3, 6.0, 9.0);
    cup.close();
    // The handles.
    cup.move_to(6.0, 4.5);
    cup.line_to(4.5, 4.5);
    cup.cubic_to(3.1, 4.5, 2.0, 5.6, 2.0, 7.0);
    cup.cubic_to(2.0, 8.4, 3.1, 9.5, 4.5, 9.5);
    cup.line_to(6.3, 9.5);
    cup.move_to(18.0, 4.5);
    cup.line_to(19.5, 4.5);
    cup.cubic_to(20.9, 4.5, 22.0, 5.6, 22.0, 7.0);
    cup.cubic_to(22.0, 8.4, 20.9, 9.5, 19.5, 9.5);
    cup.line_to(17.7, 9.5);
    // The stem and the foot.
    cup.move_to(10.5, 15.0);
    cup.line_to(10.5, 17.5);
    cup.cubic_to(10.5, 18.5, 7.0, 19.5, 7.0, 21.5);
    cup.move_to(13.5, 15.0);
    cup.line_to(13.5, 17.5);
    cup.cubic_to(13.5, 18.5, 17.0, 19.5, 17.0, 21.5);
    cup.move_to(5.0, 21.5);
    cup.line_to(19.0, 21.5);

    let mut star = PathBuilder::new();
    let (cx, cy, outer, inner) = (12.0_f32, 8.6_f32, 3.4_f32, 1.45_f32);
    for point in 0..10 {
        let radius = if point % 2 == 0 { outer } else { inner };
        let angle = -std::f32::consts::FRAC_PI_2 + point as f32 * std::f32::consts::PI / 5.0;
        let (x, y) = (cx + radius * angle.cos(), cy + radius * angle.sin());
        if point == 0 {
            star.move_to(x, y);
        } else {
            star.line_to(x, y);
        }
    }
    star.close();
    (cup.finish(), star.finish())
}

/// A shuttlecock on a 24 grid, cork down: the skirt and its ribs as a
/// stroked path, and the cork as a filled one.
fn shuttle() -> (Option<Path>, Option<Path>) {
    let mut skirt = PathBuilder::new();
    skirt.move_to(9.0, 16.0);
    skirt.line_to(5.0, 3.0);
    skirt.line_to(19.0, 3.0);
    skirt.line_to(15.0, 16.0);
    skirt.close();
    skirt.move_to(11.0, 16.0);
    skirt.line_to(10.0, 3.0);
    skirt.move_to(13.0, 16.0);
    skirt.line_to(14.0, 3.0);
    skirt.move_to(6.5, 8.0);
    skirt.line_to(17.5, 8.0);
    let mut cork = PathBuilder::new();
    cork.move_to(8.5, 16.0);
    cork.line_to(15.5, 16.0);
    cork.cubic_to(15.5, 20.5, 8.5, 20.5, 8.5, 16.0);
    cork.close();
    (skirt.finish(), cork.finish())
}

/// Draws a 24-grid icon `size` pixels square with its centre at `(cx,
/// cy)`, turned `degrees` clockwise.
fn draw_icon(
    canvas: &mut Pixmap,
    (stroked, filled): (Option<Path>, Option<Path>),
    (cx, cy): (f32, f32),
    size: f32,
    degrees: f32,
    colour: Color,
) {
    let scale = size / 24.0;
    let transform = Transform::from_translate(-12.0, -12.0)
        .post_rotate(degrees)
        .post_scale(scale, scale)
        .post_translate(cx, cy);
    let paint = solid(colour);
    if let Some(path) = &stroked {
        let stroke = Stroke {
            width: 1.7,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        canvas.stroke_path(path, &paint, &stroke, transform, None);
    }
    if let Some(path) = &filled {
        canvas.fill_path(path, &paint, FillRule::Winding, transform, None);
    }
}

/// A soft copy of `path` - the board's shadow, a glow - drawn under what
/// comes next: painted on a canvas just big enough, blurred, laid down.
fn soften(
    canvas: &mut Pixmap,
    path: &Path,
    paint: &Paint<'_>,
    stroke: Option<&Stroke>,
    radius: f32,
    offset: (f32, f32),
) {
    let bounds = path.bounds();
    let margin = radius * 3.0 + stroke.map_or(0.0, |stroke| stroke.width);
    let (w, h) = (
        (bounds.width() + 2.0 * margin).ceil() as u32,
        (bounds.height() + 2.0 * margin).ceil() as u32,
    );
    let Some(mut layer) = Pixmap::new(w.max(1), h.max(1)) else {
        return;
    };
    let shift = Transform::from_translate(margin - bounds.left(), margin - bounds.top());
    match stroke {
        Some(stroke) => layer.stroke_path(path, paint, stroke, shift, None),
        None => layer.fill_path(path, paint, FillRule::Winding, shift, None),
    }
    blur(&mut layer, radius.round().max(1.0) as usize);
    canvas.draw_pixmap(
        (bounds.left() - margin + offset.0).round() as i32,
        (bounds.top() - margin + offset.1).round() as i32,
        layer.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

/// The board on its canvas, shrunk to fit the canvas's width when it would
/// run past it: the canvas is the frame, and what runs past its edge is
/// never drawn - on a portrait frame a board at its usual size is wider
/// than the picture, and lost both its ends. A board's width follows its
/// height, so it is measured once and painted again at the height that
/// fits.
fn paint_board(
    fonts: &Fonts,
    board: &Scoreboard,
    width: u32,
    height: u32,
) -> Result<(Pixmap, Block), Error> {
    let painted = paint_board_at(fonts, board, width, height)?;
    let room = width as f32 * 0.96;
    let wide = painted.1.0 as f32;
    if wide <= room {
        return Ok(painted);
    }
    let fitted = Scoreboard {
        size: board.size * f64::from(room / wide),
        ..board.clone()
    };
    paint_board_at(fonts, &fitted, width, height)
}

fn paint_board_at(
    fonts: &Fonts,
    board: &Scoreboard,
    width: u32,
    height: u32,
) -> Result<(Pixmap, Block), Error> {
    let mut canvas = Pixmap::new(width, height).ok_or(Error::Canvas(width, height))?;
    let spec = board.board.spec();
    let tall = (board.size.clamp(0.03, 1.0) as f32) * height as f32;
    let ink = colour(&board.ink).unwrap_or(Color::WHITE);
    let panel = colour(&board.panel).unwrap_or(Color::BLACK);
    let accent = colour(&board.accent).unwrap_or(Color::WHITE);
    let accent_ink = colour(&board.accent_ink).unwrap_or(Color::BLACK);
    let score = colour(&board.score).unwrap_or(ink);
    let home = colour(&board.home_color).unwrap_or(accent);
    let away = colour(&board.away_color).unwrap_or(accent);
    let serve = colour(&board.serve_color).unwrap_or(accent);
    let family = board.font_family.as_str();
    let weight = board.font_weight;
    let say = |text: &str, cap: f32| label(fonts, text, family, weight, cap);
    let names = (
        board.home_name.to_uppercase(),
        board.away_name.to_uppercase(),
    );

    if spec.inline {
        // One row carries what two do, so it is about half as tall for
        // the same size: the boards weigh the same on the picture.
        return paint_inline(&mut canvas, board, &spec, tall * 0.55, names, say)
            .map(|block| (canvas, block));
    }

    let header = spec.header && !board.title.trim().is_empty();
    let header_h = if header { tall * 0.24 } else { 0.0 };
    let body_h = tall - header_h;
    let pad = body_h * 0.1;
    let row_h = (body_h - 2.0 * pad) / 2.0;

    // The words, shaped once, so the columns can be measured.
    let name_cap = row_h * 0.44;
    let points_cap = row_h * 0.6;
    let games_cap = row_h * 0.42;
    let home_label = say(&names.0, name_cap)?;
    let away_label = say(&names.1, name_cap)?;
    let home_points = say(&board.home_points.to_string(), points_cap)?;
    let away_points = say(&board.away_points.to_string(), points_cap)?;
    // The widest score the column will have to hold, so it never moves.
    let points_room = say("00", points_cap)?
        .width
        .max(home_points.width)
        .max(away_points.width);
    let home_games = say(&board.home_games.to_string(), games_cap)?;
    let away_games = say(&board.away_games.to_string(), games_cap)?;
    let title = if header {
        Some(say(&board.title.to_uppercase(), header_h * 0.42)?)
    } else {
        None
    };

    // The columns, left to right: (left edge, width) each.
    let icon = spec.icon && board.show_icon;
    let mut x = pad * 1.2;
    let icon_col = icon.then(|| {
        let w = body_h * 0.62;
        let col = (x, w);
        x += w + pad;
        col
    });
    let bar_w = match spec.bar {
        Bar::Block => row_h * 0.68,
        Bar::Stripe => row_h * 0.14,
        Bar::Dot => row_h * 0.3,
    };
    let bar_col = (x, bar_w);
    x += bar_w
        + match spec.bar {
            Bar::Block => pad * 1.3,
            _ => pad * 0.9,
        };
    let name_w = home_label
        .width
        .max(away_label.width)
        .clamp(row_h * 2.4, row_h * 9.0);
    let name_col = (x, name_w);
    x += name_w + pad * 1.2;
    let serve_col = board.show_serve.then(|| {
        let w = row_h * 0.62;
        let col = (x, w);
        x += w + pad * 0.5;
        col
    });
    let games_col = board.show_games.then(|| {
        let w = if spec.games_box {
            row_h * 0.95
        } else {
            home_games.width.max(away_games.width).max(row_h * 0.5)
        };
        let col = (x, w);
        x += w + pad * if spec.games_box { 0.7 } else { 0.9 };
        col
    });
    let points_w = if spec.points_box {
        points_room + row_h * 0.5
    } else {
        points_room + row_h * 0.1
    };
    let points_col = (x, points_w);
    x += points_w + pad * 1.2;
    let board_w = x;
    let title_w = title
        .as_ref()
        .map_or(0.0, |title| title.width + header_h * 1.2);
    let board_w = board_w.max(title_w);

    // The board sits centred on the canvas: the clip's position is its
    // middle. Every column so far was measured from its left edge.
    let left = (width as f32 - board_w) / 2.0;
    let top = (height as f32 - tall) / 2.0;
    let radius = spec.radius * tall;
    let body_top = top + header_h;
    let rows = [body_top + pad + row_h / 2.0, body_top + pad + row_h * 1.5];
    let at = |(x, w): (f32, f32)| (left + x, w);

    let outline = rounded(left, top, board_w, tall, radius).ok_or(Error::Canvas(width, height))?;
    // A soft shadow under every board that is not glass.
    if panel.alpha() > 0.9 {
        soften(
            &mut canvas,
            &outline,
            &solid(Color::from_rgba8(0, 0, 0, 90)),
            None,
            tall * 0.06,
            (0.0, tall * 0.03),
        );
    }
    if spec.glow {
        let stroke = Stroke {
            width: tall * 0.035,
            ..Stroke::default()
        };
        soften(
            &mut canvas,
            &outline,
            &solid(accent),
            Some(&stroke),
            tall * 0.06,
            (0.0, 0.0),
        );
    }
    canvas.fill_path(
        &outline,
        &solid(panel),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    if let Some(title) = &title
        && let Some(strip) = half_rounded(left, top, board_w, header_h, radius, true)
    {
        canvas.fill_path(
            &strip,
            &solid(accent),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        draw_label(
            &mut canvas,
            title,
            (left + header_h * 0.6, board_w - header_h * 1.2),
            top + header_h / 2.0,
            Justify::Start,
            accent_ink,
        );
    }
    if spec.glow {
        canvas.stroke_path(
            &outline,
            &solid(accent),
            &Stroke {
                width: (tall * 0.012).max(1.0),
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    }
    if spec.court_line {
        let inset = tall * 0.045;
        if let Some(line) = rounded(
            left + inset,
            top + inset,
            board_w - 2.0 * inset,
            tall - 2.0 * inset,
            (radius - inset).max(0.0),
        ) {
            canvas.stroke_path(
                &line,
                &solid(faded(ink, 0.85)),
                &Stroke {
                    width: (tall * 0.012).max(1.0),
                    ..Stroke::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
    if spec.divider {
        let (x0, _) = at(name_col);
        let x1 = left + board_w - pad * 1.2;
        if let Some(line) = rect(
            x0,
            body_top + body_h / 2.0 - 0.5,
            x1 - x0,
            1.0_f32.max(tall * 0.006),
        ) {
            canvas.fill_rect(line, &solid(faded(ink, 0.15)), Transform::identity(), None);
        }
    }

    if let Some(col) = icon_col {
        let (x, w) = at(col);
        let tint = if spec.icon_accent { accent } else { ink };
        draw_icon(
            &mut canvas,
            trophy(),
            (x + w / 2.0, body_top + body_h / 2.0),
            w,
            0.0,
            tint,
        );
    }

    // Each side's colour.
    let (bx, bw) = at(bar_col);
    for (row, tint) in rows.iter().zip([home, away]) {
        let paint = solid(tint);
        match spec.bar {
            Bar::Block => {
                if let Some(block) = rect(bx, row - row_h / 2.0, bw, row_h) {
                    canvas.fill_rect(block, &paint, Transform::identity(), None);
                }
            }
            Bar::Stripe => {
                if let Some(stripe) = rounded(bx, row - row_h * 0.36, bw, row_h * 0.72, bw / 2.0) {
                    canvas.fill_path(
                        &stripe,
                        &paint,
                        FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
            Bar::Dot => {
                let mut dot = PathBuilder::new();
                dot.push_circle(bx + bw / 2.0, *row, bw / 2.0);
                if let Some(dot) = dot.finish() {
                    canvas.fill_path(&dot, &paint, FillRule::Winding, Transform::identity(), None);
                }
            }
        }
    }

    // The names.
    for (row, name) in rows.iter().zip([&home_label, &away_label]) {
        draw_label(&mut canvas, name, at(name_col), *row, Justify::Start, ink);
    }

    // The shuttle beside whoever serves.
    if let (Some(col), Some(home_serves)) = (serve_col, board.serving) {
        let (x, w) = at(col);
        let row = if home_serves { rows[0] } else { rows[1] };
        draw_icon(&mut canvas, shuttle(), (x + w / 2.0, row), w, 35.0, serve);
    }

    // The games, on their box or as dim figures.
    if let Some(col) = games_col {
        let (x, w) = at(col);
        let tint = if spec.games_box {
            let span = rows[1] - rows[0] + row_h;
            if let Some(boxed) = rounded(x, rows[0] - row_h / 2.0, w, span, row_h * 0.08) {
                canvas.fill_path(
                    &boxed,
                    &solid(accent),
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
            if let Some(seam) = rect(
                x,
                body_top + body_h / 2.0 - 0.5,
                w,
                1.0_f32.max(tall * 0.006),
            ) {
                canvas.fill_rect(seam, &solid(faded(panel, 0.6)), Transform::identity(), None);
            }
            accent_ink
        } else {
            faded(ink, 0.6)
        };
        for (row, games) in rows.iter().zip([&home_games, &away_games]) {
            draw_label(&mut canvas, games, (x, w), *row, Justify::Middle, tint);
        }
    }

    // The points.
    let (px, pw) = at(points_col);
    let tint = if spec.points_box {
        let span = rows[1] - rows[0] + row_h;
        if let Some(boxed) = rounded(px, rows[0] - row_h / 2.0, pw, span, row_h * 0.12) {
            canvas.fill_path(
                &boxed,
                &solid(accent),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        accent_ink
    } else {
        score
    };
    for (row, points) in rows.iter().zip([&home_points, &away_points]) {
        draw_label(&mut canvas, points, (px, pw), *row, Justify::Middle, tint);
    }

    Ok((
        canvas,
        (
            board_w.ceil().max(1.0) as u32,
            tall.ceil().max(1.0) as u32,
            0,
            0,
        ),
    ))
}

/// The one-row board: each side's colour and name at its own end, the
/// games beside them, and the score on a box in the middle.
fn paint_inline(
    canvas: &mut Pixmap,
    board: &Scoreboard,
    spec: &Spec,
    tall: f32,
    names: (String, String),
    say: impl Fn(&str, f32) -> Result<Label, Error>,
) -> Result<Block, Error> {
    let (width, height) = (canvas.width() as f32, canvas.height() as f32);
    let ink = colour(&board.ink).unwrap_or(Color::WHITE);
    let panel = colour(&board.panel).unwrap_or(Color::BLACK);
    let accent = colour(&board.accent).unwrap_or(Color::WHITE);
    let accent_ink = colour(&board.accent_ink).unwrap_or(Color::BLACK);
    let home = colour(&board.home_color).unwrap_or(accent);
    let away = colour(&board.away_color).unwrap_or(accent);
    let serve = colour(&board.serve_color).unwrap_or(accent);

    let pad = tall * 0.2;
    let home_label = say(&names.0, tall * 0.3)?;
    let away_label = say(&names.1, tall * 0.3)?;
    let points_cap = tall * 0.38;
    let home_points = say(&board.home_points.to_string(), points_cap)?;
    let away_points = say(&board.away_points.to_string(), points_cap)?;
    let points_room = say("00", points_cap)?
        .width
        .max(home_points.width)
        .max(away_points.width);
    let dash = say("-", points_cap)?;
    let games_cap = tall * 0.26;
    let home_games = say(&board.home_games.to_string(), games_cap)?;
    let away_games = say(&board.away_games.to_string(), games_cap)?;

    // Both halves are as wide as the wider, so the score is the board's
    // middle whatever the names.
    let stripe_w = tall * 0.09;
    let serve_w = if board.show_serve { tall * 0.42 } else { 0.0 };
    let name_w = home_label
        .width
        .max(away_label.width)
        .clamp(tall * 1.5, tall * 6.0);
    let games_w = if board.show_games {
        home_games.width.max(away_games.width).max(tall * 0.3)
    } else {
        0.0
    };
    let half_side = pad
        + stripe_w
        + pad * 0.6
        + if board.show_serve {
            serve_w + pad * 0.3
        } else {
            0.0
        }
        + name_w
        + pad * 0.8
        + if board.show_games {
            games_w + pad * 0.6
        } else {
            0.0
        };
    let gap = pad * 0.35;
    let box_w = points_room * 2.0 + dash.width + gap * 2.0 + pad * 2.2;
    let board_w = half_side * 2.0 + box_w;

    let left = (width - board_w) / 2.0;
    let top = (height - tall) / 2.0;
    let cy = top + tall / 2.0;
    let radius = spec.radius * tall;
    let outline = rounded(left, top, board_w, tall, radius)
        .ok_or(Error::Canvas(width as u32, height as u32))?;
    if panel.alpha() > 0.9 {
        soften(
            canvas,
            &outline,
            &solid(Color::from_rgba8(0, 0, 0, 90)),
            None,
            tall * 0.08,
            (0.0, tall * 0.04),
        );
    }
    canvas.fill_path(
        &outline,
        &solid(panel),
        FillRule::Winding,
        Transform::identity(),
        None,
    );

    // The score box in the middle, a little inset from the board's edges.
    let box_left = left + half_side;
    let inset = tall * 0.12;
    if let Some(boxed) = rounded(
        box_left,
        top + inset,
        box_w,
        tall - 2.0 * inset,
        (radius - inset).max(tall * 0.06),
    ) {
        canvas.fill_path(
            &boxed,
            &solid(accent),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    let inner = box_left + pad * 1.1;
    draw_label(
        canvas,
        &home_points,
        (inner, points_room),
        cy,
        Justify::Middle,
        accent_ink,
    );
    draw_label(
        canvas,
        &dash,
        (inner + points_room + gap, dash.width),
        cy,
        Justify::Middle,
        faded(accent_ink, 0.6),
    );
    draw_label(
        canvas,
        &away_points,
        (inner + points_room + dash.width + gap * 2.0, points_room),
        cy,
        Justify::Middle,
        accent_ink,
    );

    // Each half, mirrored: from the board's edge in, the stripe, the
    // shuttle's place, the name, the games.
    for (side, (name, games, tint)) in [
        (&home_label, &home_games, home),
        (&away_label, &away_games, away),
    ]
    .into_iter()
    .enumerate()
    {
        let mirrored = side == 1;
        // `place` turns a column measured from this half's outer edge into
        // canvas terms.
        let place = |from: f32, w: f32| {
            if mirrored {
                (left + board_w - from - w, w)
            } else {
                (left + from, w)
            }
        };
        let mut x = pad;
        let (sx, sw) = place(x, stripe_w);
        if let Some(stripe) = rounded(sx, cy - tall * 0.28, sw, tall * 0.56, sw / 2.0) {
            canvas.fill_path(
                &stripe,
                &solid(tint),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        x += stripe_w + pad * 0.6;
        if board.show_serve {
            let (vx, vw) = place(x, serve_w);
            if board.serving == Some(!mirrored) {
                draw_icon(canvas, shuttle(), (vx + vw / 2.0, cy), vw, 35.0, serve);
            }
            x += serve_w + pad * 0.3;
        }
        let (nx, nw) = place(x, name_w);
        // A name reads from its own end of the board towards the middle.
        let shown = name.width.min(nw);
        let name_x = if mirrored { nx + nw - shown } else { nx };
        draw_label(canvas, name, (name_x, shown), cy, Justify::Start, ink);
        x += name_w + pad * 0.8;
        if board.show_games {
            let col = place(x, games_w);
            draw_label(canvas, games, col, cy, Justify::Middle, faded(ink, 0.6));
        }
    }

    Ok((
        board_w.ceil().max(1.0) as u32,
        tall.ceil().max(1.0) as u32,
        0,
        0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(look: Board) -> Scoreboard {
        Scoreboard {
            board: look,
            size: 0.2,
            font_family: "Bebas Neue".to_owned(),
            font_weight: 400.0,
            title: "Final".to_owned(),
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
            show_icon: true,
            show_games: true,
            show_serve: true,
            home_points: 9,
            away_points: 23,
            home_games: 2,
            away_games: 2,
            serving: Some(true),
        }
    }

    #[test]
    fn every_look_paints_a_board_centred_at_its_height() {
        let fonts = Fonts::new();
        for look in [
            Board::Classic,
            Board::Broadcast,
            Board::Minimal,
            Board::Neon,
            Board::Court,
            Board::Ocean,
            Board::Sunrise,
            Board::Slate,
            Board::Ink,
            Board::Tournament,
        ] {
            let frame = render_scoreboard_frame(&fonts, &board(look), 640, 360).expect("paints");
            // A fifth of 360; a one-row board a little over half that.
            let tall = if look == Board::Slate { 40 } else { 72 };
            assert_eq!(frame.block_height, tall, "{look:?}");
            assert!(frame.block_width > tall, "{look:?}: wider than tall");
            let at = |x: u32, y: u32| frame.rgba[((y * 640 + x) * 4 + 3) as usize];
            assert!(at(320, 180) > 100, "{look:?}: the board is at the middle");
            assert_eq!(at(2, 2), 0, "{look:?}: the corner is clear");
        }
    }

    /// On a frame narrower than the board, the board is shrunk to fit
    /// rather than cut at both ends.
    #[test]
    fn a_board_wider_than_a_portrait_frame_is_shrunk_to_fit_it() {
        let fonts = Fonts::new();
        for look in [Board::Classic, Board::Slate, Board::Tournament] {
            let frame = render_scoreboard_frame(&fonts, &board(look), 360, 640).expect("paints");
            assert!(
                frame.block_width <= 346,
                "{look:?}: {} wide",
                frame.block_width
            );
            assert!(
                frame.block_width > 300,
                "{look:?}: and no smaller than it must be"
            );
            let at = |x: u32, y: u32| frame.rgba[((y * 360 + x) * 4 + 3) as usize];
            assert!(at(180, 320) > 100, "{look:?}: the board is at the middle");
        }
    }

    #[test]
    fn the_board_does_not_change_width_as_the_score_climbs() {
        let fonts = Fonts::new();
        let mut early = board(Board::Classic);
        early.home_points = 1;
        let mut late = board(Board::Classic);
        late.home_points = 21;
        let a = render_scoreboard_frame(&fonts, &early, 640, 360).expect("paints");
        let b = render_scoreboard_frame(&fonts, &late, 640, 360).expect("paints");
        assert_eq!(a.block_width, b.block_width);
        let png = render_scoreboard(&fonts, &late, 64, 36).expect("encodes");
        assert!(png.png.starts_with(b"\x89PNG"));
    }
}
