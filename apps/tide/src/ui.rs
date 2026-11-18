// The screen, as LVGL batches: no Api, so the host tests validate every batch against the loader's own checker.
//
// Layout (466 × 466, round): the clock at the top, a turning colour disc, the track title and artist (the loader
// writes them, `media_bind`), frosted round buttons for previous, play/pause and next with a spinning ring around
// play/pause, and a hint line at the bottom. Behind it all, three soft "Ocean" glows: each is a stack of
// see-through circles (LVGL has no blur), so the edges fade out.
//
// Icons are drawn from rectangles: LVGL apps have no image source. A triangle is a stack of 4 px rows.

use cmf_wp2_app::lv::{Align, Batch, Class, Ev, Flag, LongMode, LvError, Obj, Prop, ROOT, StockFont};
use cmf_wp2_app::stock::{self, CIRCLE, INDICATOR, KNOB, PRESSED};

/// RGB888 to the RGB565 the style records take.
pub const fn rgb(r: u8, g: u8, b: u8) -> i32 {
    (((r as i32) >> 3) << 11) | (((g as i32) >> 2) << 5) | ((b as i32) >> 3)
}

// Ocean: teal, blue, green. Dark enough that white text stays readable on top of all three.
pub const GLOW: [i32; 3] = [rgb(14, 124, 134), rgb(31, 79, 184), rgb(39, 179, 154)];
const DISC: i32 = rgb(24, 112, 128);
const DISC_OFF: i32 = rgb(38, 38, 46);
const SOFT: i32 = rgb(200, 226, 228);
const DIM: i32 = rgb(150, 150, 160);
const WHITE: i32 = stock::WHITE;
const BLACK: i32 = stock::BLACK;

/// Font slots: 0 is the system font (title), 1 the clock and hints, 2 the artist.
pub const FONT_SMALL: u8 = 1;
pub const FONT_ARTIST: u8 = 2;
pub const FONTS: u8 = 0b111;

/// The glows: home centre and radius. Each is `LAYERS` circles, `LAYER_OPA` each, shrinking towards the middle.
pub const GLOWS: [(i16, i16, i32); 3] = [(190, 170, 210), (290, 280, 190), (220, 330, 170)];
pub const LAYERS: usize = 4;
const LAYER_OPA: i32 = 44;

/// Opacity of the whole glow while playing, paused, and with nothing to play.
pub const GLOW_ON: i32 = 255;
pub const GLOW_PAUSED: i32 = 140;
pub const GLOW_OFF: i32 = 70;

#[derive(Clone, Copy, Debug)]
pub struct Ui {
    pub glows: [Obj; 3],
    pub clock: Obj,
    pub disc: Obj,
    pub shine: Obj,
    pub title: Obj,
    pub artist: Obj,
    pub empty: Obj,
    pub empty_hint: Obj,
    pub ring: Obj,
    pub prev: Obj,
    pub play: Obj,
    pub next: Obj,
    pub play_icon: Obj,
    pub pause_icon: Obj,
    pub hint: Obj,
}

impl Ui {
    /// Every handle the root until the build fills it in.
    pub const EMPTY: Ui = Ui {
        glows: [ROOT; 3],
        clock: ROOT,
        disc: ROOT,
        shine: ROOT,
        title: ROOT,
        artist: ROOT,
        empty: ROOT,
        empty_hint: ROOT,
        ring: ROOT,
        prev: ROOT,
        play: ROOT,
        next: ROOT,
        play_icon: ROOT,
        pause_icon: ROOT,
        hint: ROOT,
    };
}

/// Batch 1, on its own: the fonts (a missing one stops the batch).
pub fn fonts(b: &mut Batch) -> Result<(), LvError> {
    stock::font(b)?;
    b.font(FONT_SMALL, StockFont::F24M)?;
    b.font(FONT_ARTIST, StockFont::F24R)
}

fn plain(b: &mut Batch, parent: Obj) -> Result<Obj, LvError> {
    let o = b.create(Class::Obj, parent)?;
    b.flags(o, &[], &[Flag::Clickable, Flag::Scrollable])?;
    Ok(o)
}

fn place(b: &mut Batch, o: Obj, x: i32, y: i32, w: i32, h: i32) -> Result<(), LvError> {
    b.size(o, w, h)?;
    b.align(o, Align::TopLeft, x as i16, y as i16)
}

fn fill(b: &mut Batch, o: Obj, colour: i32, opa: i32) -> Result<(), LvError> {
    b.style(o, Prop::BgColor, 0, colour)?;
    b.style(o, Prop::BgOpa, 0, opa)
}

fn circle(b: &mut Batch, parent: Obj, x: i32, y: i32, d: i32, colour: i32, opa: i32) -> Result<Obj, LvError> {
    let o = plain(b, parent)?;
    place(b, o, x, y, d, d)?;
    fill(b, o, colour, opa)?;
    b.style(o, Prop::Radius, 0, CIRCLE)?;
    Ok(o)
}

/// Batch 2: the black screen and the three glows.
pub fn background(b: &mut Batch, ui: &mut Ui) -> Result<(), LvError> {
    stock::theme(b)?;
    fill(b, ROOT, BLACK, 255)?;
    for (i, &(cx, cy, r)) in GLOWS.iter().enumerate() {
        let g = plain(b, ROOT)?;
        place(b, g, cx as i32 - r, cy as i32 - r, 2 * r, 2 * r)?;
        b.style(g, Prop::BgOpa, 0, 0)?;
        for k in 0..LAYERS as i32 {
            let rr = r * (100 - 18 * k) / 100;
            circle(b, g, r - rr, r - rr, 2 * rr, GLOW[i], LAYER_OPA)?;
        }
        ui.glows[i] = g;
    }
    Ok(())
}

fn label(b: &mut Batch, x: i32, y: i32, w: i32, h: i32, font: u8, colour: i32, text: &str) -> Result<Obj, LvError> {
    let l = b.create(Class::Label, ROOT)?;
    place(b, l, x, y, w, h)?;
    b.style(l, Prop::TextAlign, 0, 2)?; // centre
    if font != stock::FONT {
        b.style(l, Prop::TextFont, 0, font as i32)?;
    }
    if colour != WHITE {
        b.style(l, Prop::TextColor, 0, colour)?;
    }
    b.text(l, text)?;
    Ok(l)
}

/// A see-through arc: `opa` on the background part, nothing on the indicator and the knob, not clickable.
fn ring_arc(b: &mut Batch, parent: Obj, x: i32, y: i32, d: i32, width: i32, opa: i32, sweep: u16) -> Result<Obj, LvError> {
    let a = b.create(Class::Arc, parent)?;
    place(b, a, x, y, d, d)?;
    b.flags(a, &[], &[Flag::Clickable, Flag::Scrollable])?;
    b.style(a, Prop::ArcColor, 0, WHITE)?;
    b.style(a, Prop::ArcOpa, 0, opa)?;
    b.style(a, Prop::ArcWidth, 0, width)?;
    b.style(a, Prop::ArcRounded, 0, 1)?;
    b.style(a, Prop::ArcOpa, INDICATOR, 0)?;
    b.style(a, Prop::BgOpa, KNOB, 0)?;
    b.angles(a, 0, sweep, 270)?;
    Ok(a)
}

/// Batch 3: clock, disc, the track labels and the "nothing playing" pair.
pub fn middle(b: &mut Batch, ui: &mut Ui) -> Result<(), LvError> {
    ui.clock = label(b, 133, 22, 200, 30, FONT_SMALL, WHITE, "")?;
    ui.disc = circle(b, ROOT, 158, 58, 150, DISC, 255)?;
    ui.shine = ring_arc(b, ui.disc, 12, 12, 126, 6, 150, 80)?;
    circle(b, ui.disc, 59, 59, 32, BLACK, 255)?;
    ui.title = label(b, 63, 220, 340, 42, stock::FONT, WHITE, "")?;
    b.long_mode(ui.title, LongMode::ScrollCircular)?;
    ui.artist = label(b, 83, 264, 300, 32, FONT_ARTIST, SOFT, "")?;
    b.long_mode(ui.artist, LongMode::Dot)?;
    ui.empty = label(b, 63, 222, 340, 42, stock::FONT, DIM, "Nothing playing")?;
    ui.empty_hint = label(b, 63, 264, 340, 32, FONT_ARTIST, DIM, "Start music on your phone")?;
    Ok(())
}

/// A frosted round button that sends CLICKED.
fn button(b: &mut Batch, x: i32, y: i32, d: i32, opa: i32) -> Result<Obj, LvError> {
    let o = b.create(Class::Btn, ROOT)?;
    place(b, o, x, y, d, d)?;
    b.flags(o, &[], &[Flag::Scrollable])?;
    fill(b, o, WHITE, opa)?;
    b.style(o, Prop::BgOpa, PRESSED, opa + 50)?;
    b.style(o, Prop::Radius, 0, CIRCLE)?;
    b.style(o, Prop::BorderWidth, 0, 0)?;
    b.events(o, &[Ev::Clicked])?;
    Ok(o)
}

/// A white triangle `w` × `h` at (x, y) in `parent`, pointing right or left, as rows of 4 px.
fn triangle(b: &mut Batch, parent: Obj, x: i32, y: i32, w: i32, h: i32, right: bool) -> Result<(), LvError> {
    let rows = h / 4;
    for i in 0..rows {
        let mid = 2 * i + 1 - rows; // -(rows-1)..=rows-1, in half-rows from the middle
        let rw = (w * (rows - mid.abs()) / rows).max(2);
        let rx = if right { x } else { x + w - rw };
        let r = plain(b, parent)?;
        place(b, r, rx, y + 4 * i, rw, 4)?;
        fill(b, r, WHITE, 255)?;
    }
    Ok(())
}

/// Batch 4: the ring around play/pause and the three buttons with the play/pause icons.
pub fn controls(b: &mut Batch, ui: &mut Ui) -> Result<(), LvError> {
    ring_arc(b, ROOT, 183, 303, 100, 5, 60, 360)?;
    ui.ring = ring_arc(b, ROOT, 183, 303, 100, 5, 255, 80)?;
    ui.prev = button(b, 92, 318, 70, 46)?;
    ui.play = button(b, 194, 314, 78, 66)?;
    ui.next = button(b, 304, 318, 70, 46)?;
    ui.play_icon = plain(b, ui.play)?;
    place(b, ui.play_icon, 28, 23, 28, 32)?;
    triangle(b, ui.play_icon, 0, 0, 26, 32, true)?;
    ui.pause_icon = plain(b, ui.play)?;
    place(b, ui.pause_icon, 25, 25, 28, 28)?;
    for x in [0, 18] {
        let bar = plain(b, ui.pause_icon)?;
        place(b, bar, x, 0, 9, 28)?;
        fill(b, bar, WHITE, 255)?;
        b.style(bar, Prop::Radius, 0, 2)?;
    }
    b.flags(ui.pause_icon, &[Flag::Hidden], &[])?;
    Ok(())
}

/// Batch 5: the previous and next icons (two triangles each) and the hint line.
pub fn skip_icons(b: &mut Batch, ui: &mut Ui) -> Result<(), LvError> {
    triangle(b, ui.prev, 21, 23, 14, 24, false)?;
    triangle(b, ui.prev, 35, 23, 14, 24, false)?;
    triangle(b, ui.next, 21, 23, 14, 24, true)?;
    triangle(b, ui.next, 35, 23, 14, 24, true)?;
    ui.hint = label(b, 123, 406, 220, 30, FONT_SMALL, SOFT, "")?;
    Ok(())
}

/// The screen's state: what the phone says, and whether the app's own view is paused (a popup over it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    pub available: bool,
    pub playing: bool,
}

/// Shows `look`: the track or "nothing playing", play or pause, the glow's strength, the disc's colour.
pub fn show(b: &mut Batch, ui: &Ui, look: Look) -> Result<(), LvError> {
    let hide = |b: &mut Batch, o: Obj, hidden: bool| {
        if hidden { b.flags(o, &[Flag::Hidden], &[]) } else { b.flags(o, &[], &[Flag::Hidden]) }
    };
    hide(b, ui.title, !look.available)?;
    hide(b, ui.artist, !look.available)?;
    hide(b, ui.empty, look.available)?;
    hide(b, ui.empty_hint, look.available)?;
    let playing = look.available && look.playing;
    hide(b, ui.play_icon, playing)?;
    hide(b, ui.pause_icon, !playing)?;
    hide(b, ui.shine, !look.available)?;
    let opa = if !look.available { GLOW_OFF } else if playing { GLOW_ON } else { GLOW_PAUSED };
    for g in ui.glows {
        b.style(g, Prop::Opa, 0, opa)?;
    }
    b.style(ui.disc, Prop::BgColor, 0, if look.available { DISC } else { DISC_OFF })?;
    b.style(ui.ring, Prop::ArcOpa, 0, if playing { 255 } else { 110 })
}

/// One animation frame: the glows' offsets and the two rotations.
pub fn frame(b: &mut Batch, ui: &Ui, offsets: [(i16, i16); 3], ring_deg: u16, disc_deg: u16) -> Result<(), LvError> {
    for (g, (dx, dy)) in ui.glows.iter().zip(offsets) {
        b.style(*g, Prop::TranslateX, 0, dx as i32)?;
        b.style(*g, Prop::TranslateY, 0, dy as i32)?;
    }
    b.angles(ui.ring, 0, 80, (270 + ring_deg) % 360)?;
    b.angles(ui.shine, 0, 80, (270 + disc_deg) % 360)
}

/// "HH:MM" into `buf`.
pub fn clock_text(hour: u8, minute: u8, buf: &mut [u8; 5]) -> &str {
    *buf = [b'0' + hour / 10 % 10, b'0' + hour % 10, b':', b'0' + minute / 10 % 10, b'0' + minute % 10];
    core::str::from_utf8(buf).unwrap_or("")
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use cmf_wp2_app::lv::BUF;
    use std::boxed::Box;
    use wp2_core::lvcmd::{self, Table};

    fn batch() -> Batch {
        Batch::with_buf(Box::leak(Box::new([0; BUF])))
    }

    fn apply(t: &mut Table, records: &[u8]) {
        for r in lvcmd::records(records) {
            lvcmd::apply(t, &r.unwrap());
        }
    }

    // Each stage in its own batch, validated and applied in launch order, on one Batch (it owns the handles).
    fn launch() -> (Batch, Ui, Table) {
        let mut f = batch();
        fonts(&mut f).unwrap();
        assert_eq!(lvcmd::validate(f.bytes(), &Table::new(), 0), Ok(()));
        let mut t = Table::new();
        let mut b = batch();
        let mut ui = Ui::EMPTY;
        let stages: [fn(&mut Batch, &mut Ui) -> Result<(), LvError>; 4] = [background, middle, controls, skip_icons];
        for (i, stage) in stages.iter().enumerate() {
            let at = b.bytes().len();
            stage(&mut b, &mut ui).unwrap_or_else(|e| panic!("stage {i}: {e:?}"));
            let rec = b.bytes()[at..].to_vec();
            std::println!("stage {i}: {} B", rec.len());
            assert_eq!(lvcmd::validate(&rec, &t, FONTS), Ok(()), "stage {i}");
            apply(&mut t, &rec);
            b.clear();
        }
        (b, ui, t)
    }

    #[test]
    fn every_batch_is_valid_and_fits() {
        let (b, _, _) = launch();
        assert_eq!(b.bytes().len(), 0);
    }

    #[test]
    fn the_theme_needs_the_system_font() {
        let mut b = batch();
        background(&mut b, &mut Ui::EMPTY.clone()).unwrap();
        assert!(lvcmd::validate(b.bytes(), &Table::new(), 0).is_err());
    }

    #[test]
    fn object_budget_leaves_room() {
        // the loader allows 192; the GUI heap budget is ~32-40 KB (~245 B per object measured with lv-demo)
        let mut b = batch();
        let mut ui = Ui::EMPTY;
        let mut n = 0;
        for stage in [background, middle, controls, skip_icons] {
            stage(&mut b, &mut ui).unwrap();
            b.clear();
        }
        while b.create(Class::Obj, ROOT).is_ok() {
            n += 1;
            b.clear();
        }
        let used = 192 - n;
        std::println!("objects: {used}");
        assert!(used <= 110, "{used} objects");
    }

    #[test]
    fn state_and_frame_batches_are_valid() {
        let (mut b, ui, mut t) = launch();
        for look in [
            Look { available: false, playing: false },
            Look { available: true, playing: true },
            Look { available: true, playing: false },
        ] {
            show(&mut b, &ui, look).unwrap();
            assert_eq!(lvcmd::validate(b.bytes(), &t, FONTS), Ok(()), "{look:?}");
            apply(&mut t, b.bytes());
            b.clear();
        }
        for step in 0..50u32 {
            let tms = step * crate::motion::FRAME_MS;
            let off = [0, 1, 2].map(|i| crate::motion::drift(i, tms));
            frame(
                &mut b,
                &ui,
                off,
                crate::motion::degrees(tms, crate::motion::RING_MS),
                crate::motion::degrees(tms, crate::motion::DISC_MS),
            )
            .unwrap();
            assert!(b.bytes().len() < 200, "frame batch {} B", b.bytes().len());
            assert_eq!(lvcmd::validate(b.bytes(), &t, FONTS), Ok(()));
            b.clear();
        }
        b.text(ui.clock, "09:41").unwrap();
        b.text(ui.hint, "Volume +").unwrap();
        assert_eq!(lvcmd::validate(b.bytes(), &t, FONTS), Ok(()));
    }

    #[test]
    fn everything_stays_on_the_round_screen() {
        // the text boxes and buttons sit inside the 233 px disc
        let ins = |x: i32, y: i32| {
            let (dx, dy) = (x - 233, y - 233);
            dx * dx + dy * dy <= 233 * 233
        };
        for (x, y, w, h) in [
            (133, 22, 200, 30),
            (158, 58, 150, 150),
            (63, 220, 340, 42),
            (83, 264, 300, 32),
            (92, 318, 70, 70),
            (194, 314, 78, 78),
            (304, 318, 70, 70),
            (183, 303, 100, 100),
            (123, 406, 220, 30),
        ] {
            // the midpoints of the four edges (round buttons and centred text never reach their box corners)
            assert!(ins(x + w / 2, y) && ins(x + w / 2, y + h) && ins(x, y + h / 2) && ins(x + w, y + h / 2), "{x},{y}");
        }
    }

    #[test]
    fn clock() {
        let mut buf = [0; 5];
        assert_eq!(clock_text(9, 5, &mut buf), "09:05");
        assert_eq!(clock_text(23, 59, &mut buf), "23:59");
    }

    #[test]
    fn colours() {
        assert_eq!(rgb(255, 255, 255), 0xFFFF);
        assert_eq!(rgb(0, 0, 0), 0);
        assert_eq!(rgb(255, 0, 0), 0xF800);
    }
}
