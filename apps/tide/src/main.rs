#![cfg_attr(any(target_os = "none", wp2_sim), no_main, cfg_attr(target_os = "none", no_std))]

// Tide: a music remote for the phone, in the Ocean glow look. The loader writes the phone's track into two labels
// (`media_bind`) and tells the app when it starts, stops or goes away (`MEDIA`); the buttons and the crown send
// media keys. While music plays the glow drifts and the disc and ring turn; paused, everything holds still.

#[cfg_attr(not(any(target_os = "none", wp2_sim)), allow(dead_code))]
mod motion;
#[cfg_attr(not(any(target_os = "none", wp2_sim)), allow(dead_code))]
mod ui;

#[cfg(not(any(target_os = "none", wp2_sim)))]
fn main() {}

#[cfg(any(target_os = "none", wp2_sim))]
mod device {
    use crate::motion::{self, FRAME_MS};
    use crate::ui::{self, Look, Ui};
    use cmf_wp2_app::lv::{Batch, Lv};
    use cmf_wp2_app::{
        Api, BUTTON, CROWN, LV, MEDIA, MEDIA_NEXT, MEDIA_PAUSE, MEDIA_PLAY, MEDIA_PREV, PAUSE, RESUME,
    };
    use cmf_wp2_ui::haptic::{Motor, TICK};

    cmf_wp2_app::entry!(main);
    cmf_wp2_app::app_icon!();
    cmf_wp2_app::app_features!(cmf_wp2_app::PAGE);

    /// Volume up and down (cmf-watch-sdk MEDIA_KEY 5 and 6; a loader of 2026-11-16 answers -22 to them).
    const VOLUME_UP: u8 = 5;
    const VOLUME_DOWN: u8 = 6;
    /// LVGL CLICKED.
    const CLICKED: i16 = 7;
    /// How long a hint ("Volume +") stays.
    const HINT_MS: u32 = 1500;

    fn due(now: u32, at: u32) -> bool {
        (now.wrapping_sub(at) as i32) >= 0
    }

    struct App {
        api: &'static Api,
        lv: Lv,
        b: Batch,
        ui: Ui,
        look: Look,
        /// The app's own animation clock, ms; it only runs while music plays.
        t: u32,
        minute: u8,
        hint_until: Option<u32>,
    }

    impl App {
        fn submit(&mut self) {
            let _ = self.b.submit(&self.lv);
        }

        fn show(&mut self) {
            let _ = ui::show(&mut self.b, &self.ui, self.look);
            self.submit();
        }

        fn moving(&self) -> bool {
            self.look.available && self.look.playing
        }

        fn frame(&mut self) {
            let t = self.t;
            let off = [0, 1, 2].map(|i| motion::drift(i, t));
            let _ = ui::frame(
                &mut self.b,
                &self.ui,
                off,
                motion::degrees(t, motion::RING_MS),
                motion::degrees(t, motion::DISC_MS),
            );
            self.submit();
        }

        fn clock(&mut self, force: bool) {
            let Some(now) = self.api.time() else { return };
            if force || now.minute != self.minute {
                self.minute = now.minute;
                let mut buf = [0u8; 5];
                let _ = self.b.text(self.ui.clock, ui::clock_text(now.hour, now.minute, &mut buf));
                self.submit();
            }
        }

        fn hint(&mut self, text: &str) {
            let _ = self.b.text(self.ui.hint, text);
            self.submit();
            self.hint_until = Some(self.api.now().wrapping_add(HINT_MS));
        }
    }

    // true on RESUME, false on BUTTON
    fn paused(api: &Api) -> bool {
        loop {
            match api.event(u32::MAX).map(|e| e.kind) {
                Some(RESUME) => return true,
                Some(BUTTON) => return false,
                _ => {}
            }
        }
    }

    fn build(app: &mut App) -> bool {
        let stages: [fn(&mut Batch, &mut Ui) -> Result<(), cmf_wp2_app::lv::LvError>; 4] =
            [ui::background, ui::middle, ui::controls, ui::skip_icons];
        for stage in stages {
            if stage(&mut app.b, &mut app.ui).is_err() || app.b.submit(&app.lv).is_err() {
                return false;
            }
        }
        true
    }

    fn main(api: &'static Api) -> i32 {
        let Some(lv) = api.lv() else { return -95 };
        let Some(mut b) = Batch::new() else { return -1 };
        if ui::fonts(&mut b).and_then(|_| b.submit(&lv)).is_err() {
            return 1;
        }
        let mut app = App {
            api,
            lv,
            b,
            ui: Ui::EMPTY,
            look: Look { available: false, playing: false },
            t: 0,
            minute: 255,
            hint_until: None,
        };
        if !build(&mut app) {
            return 2;
        }
        app.show();
        app.clock(true);
        // -22 here means a loader older than 2026-11-16: no phone music at all
        if api.media_bind(Some(app.ui.title), Some(app.ui.artist)) != 0 {
            app.hint("Update loader");
        }
        let mut motor = Motor::new(api); // dropped on return: the motor goes off
        let mut next = api.now();
        loop {
            motor.update();
            let now = api.now();
            let mut wait = 1000u32; // the clock
            if app.moving() {
                wait = wait.min((next.wrapping_sub(now) as i32).max(0) as u32);
            }
            if let Some(h) = app.hint_until {
                wait = wait.min((h.wrapping_sub(now) as i32).max(0) as u32);
            }
            if let Some(m) = motor.wait() {
                wait = wait.min(m);
            }
            if let Some(e) = api.event(wait.max(1)) {
                match e.kind {
                    BUTTON => return 0,
                    PAUSE => {
                        motor.off();
                        if !paused(api) {
                            return 0;
                        }
                        app.clock(true);
                        next = api.now();
                    }
                    MEDIA => {
                        let was = app.moving();
                        app.look = Look { playing: e.x != 0, available: e.y != 0 };
                        app.show();
                        if app.moving() && !was {
                            next = api.now();
                        }
                    }
                    LV if e.y == CLICKED => {
                        let h = e.x as u16;
                        let key = if h == app.ui.play.0 {
                            Some(if app.moving() { MEDIA_PAUSE } else { MEDIA_PLAY })
                        } else if h == app.ui.prev.0 {
                            Some(MEDIA_PREV)
                        } else if h == app.ui.next.0 {
                            Some(MEDIA_NEXT)
                        } else {
                            None
                        };
                        if let Some(k) = key {
                            motor.play(TICK);
                            match api.media_key(k) {
                                0 if k == MEDIA_PLAY || k == MEDIA_PAUSE => {
                                    // show it at once; the next MEDIA event confirms or corrects it
                                    let was = app.moving();
                                    app.look.playing = k == MEDIA_PLAY;
                                    app.show();
                                    if app.moving() && !was {
                                        next = api.now();
                                    }
                                }
                                0 => {}
                                -19 => app.hint("No phone"),
                                _ => app.hint("Update loader"),
                            }
                        }
                    }
                    CROWN if e.delta != 0 => {
                        let up = e.delta > 0;
                        match api.media_key(if up { VOLUME_UP } else { VOLUME_DOWN }) {
                            0 => {
                                motor.play(TICK);
                                app.hint(if up { "Volume +" } else { "Volume -" });
                            }
                            -19 => app.hint("No phone"),
                            _ => app.hint("Update loader"),
                        }
                    }
                    _ => {}
                }
                continue;
            }
            let now = api.now();
            if let Some(h) = app.hint_until {
                if due(now, h) {
                    app.hint_until = None;
                    let _ = app.b.text(app.ui.hint, "");
                    app.submit();
                }
            }
            app.clock(false);
            if app.moving() && due(now, next) {
                app.t = app.t.wrapping_add(FRAME_MS);
                app.frame();
                next = next.wrapping_add(FRAME_MS);
                if due(now, next) {
                    next = now.wrapping_add(FRAME_MS); // fell behind: don't catch up in a burst
                }
            }
        }
    }
}
