//! View › Rulers › Show Video Rulers: rulers along the active artboard's top and left edges,
//! numbered in pixels from its top-left corner, beside the window's rulers.

use egui::{Pos2, Rect, Shape, vec2};
use serde_json::json;
use vectorcraft_engine::Session;
use vectorcraft_geom::Point;

use crate::{VectorcraftApp, canvas};

fn app() -> VectorcraftApp {
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("file.new", json!({"width": 400, "height": 300})).unwrap();
    app.run("artboard.new", json!({"x": 700, "y": -250, "width": 200, "height": 150})).unwrap();
    app
}

/// One canvas frame: every text label drawn, with where it is and the clip it was drawn in.
fn labels(app: &mut VectorcraftApp, ctx: &egui::Context) -> Vec<(String, Pos2, Rect)> {
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 900.0))),
        time: Some(ctx.input(|i| i.time) + 0.05),
        ..Default::default()
    };
    let mut out = ctx.run_ui(raw, |ui| canvas::show(app, ui));
    out.textures_delta.clear();
    fn walk(s: &Shape, clip: Rect, v: &mut Vec<(String, Pos2, Rect)>) {
        match s {
            Shape::Text(t) => v.push((t.galley.text().to_string(), t.pos, clip)),
            Shape::Vec(s) => s.iter().for_each(|s| walk(s, clip, v)),
            _ => {}
        }
    }
    let mut v = vec![];
    out.shapes.iter().for_each(|c| walk(&c.shape, c.clip_rect, &mut v));
    v
}

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    crate::theme::install_fonts(&ctx);
    ctx
}

/// Where artboard `index`'s top-left corner is on screen.
fn corner(app: &mut VectorcraftApp, index: usize) -> Pos2 {
    let r = app.session.doc().unwrap().doc.artboards[index].rect;
    canvas::Xf::new(app.canvas_rect.unwrap(), app.view().unwrap()).to_screen(Point::new(r.x0, r.y0))
}

/// The video rulers' zero labels: a "0" on the strip just above the artboard (top ruler) and one
/// on the strip just left of it (left ruler), each next to the artboard's corner.
fn zeros_at(texts: &[(String, Pos2, Rect)], at: Pos2) -> (bool, bool) {
    let top = texts.iter().any(|(s, p, _)| s == "0" && (p.x - 2.0 - at.x).abs() < 0.05 && p.y < at.y && p.y > at.y - 20.0);
    let left = texts.iter().any(|(s, p, _)| s == "0" && (p.y - 2.0 - at.y).abs() < 0.05 && p.x < at.x && p.x > at.x - 20.0);
    (top, left)
}

#[test]
fn show_video_rulers_menu_item_toggles_and_names_the_other_state() {
    let mut app = app();
    let view = crate::menus::menu_tree().iter().find(|(title, _)| *title == "View").map(|(_, items)| format!("{items:?}")).unwrap();
    assert!(view.contains("view.videoRulers"), "View › Rulers lists it: {view}");
    assert!(!app.ui.view.video_rulers, "off by default");
    assert_eq!(crate::menus::dynamic_label(&app, "view.videoRulers", "Show Video Rulers"), "Show Video Rulers");
    app.run("view.videoRulers", json!({})).unwrap();
    assert!(app.ui.view.video_rulers);
    assert_eq!(crate::menus::dynamic_label(&app, "view.videoRulers", "Show Video Rulers"), "Hide Video Rulers");
    assert!(!app.ui.view.rulers, "the window's rulers are a separate switch");
    app.run("view.videoRulers", json!({})).unwrap();
    assert!(!app.ui.view.video_rulers);
    assert_eq!(crate::menus::dynamic_label(&app, "view.videoRulers", "Show Video Rulers"), "Show Video Rulers");
}

#[test]
fn video_rulers_count_pixels_from_the_active_artboards_corner() {
    let mut app = app();
    let ctx = context();
    labels(&mut app, &ctx);
    let at = corner(&mut app, 0);
    assert_eq!(zeros_at(&labels(&mut app, &ctx), at), (false, false), "hidden by default");

    app.run("view.videoRulers", json!({})).unwrap();
    let texts = labels(&mut app, &ctx);
    assert_eq!(zeros_at(&texts, at), (true, true), "zero at the first artboard's top-left: {texts:?}");
    // The top ruler runs along the 400 px wide artboard: its last label reads 400 at the right edge.
    let right = canvas::Xf::new(app.canvas_rect.unwrap(), app.view().unwrap()).to_screen(Point::new(400.0, 0.0));
    assert!(texts.iter().any(|(s, p, _)| s == "400" && (p.x - 2.0 - right.x).abs() < 0.05), "{texts:?}");

    // Another artboard made active: the video rulers move to it and count from its corner.
    app.view_mut().unwrap().artboard = 1;
    labels(&mut app, &ctx);
    let at1 = corner(&mut app, 1);
    let texts = labels(&mut app, &ctx);
    assert_eq!(zeros_at(&texts, at1), (true, true), "{texts:?}");
    assert_eq!(zeros_at(&texts, corner(&mut app, 0)), (false, false), "only the active artboard has them");
}

#[test]
fn video_rulers_ignore_the_ruler_mode_and_zero_point() {
    let mut app = app();
    let ctx = context();
    app.ui.view.video_rulers = true;
    app.run("view.globalRulers", json!({"global": true})).unwrap();
    app.run("document.setRulerOrigin", json!({"x": 123, "y": 45})).unwrap();
    labels(&mut app, &ctx);
    assert_eq!(app.ruler_origin(), Point::new(123.0, 45.0));
    let at = corner(&mut app, 0);
    assert_eq!(zeros_at(&labels(&mut app, &ctx), at), (true, true));
    let doc = &app.session.doc().unwrap().doc;
    assert_eq!(doc.video_ruler_origin(0), Some(Point::ZERO));
    assert_eq!(doc.video_ruler_origin(1), Some(Point::new(700.0, -250.0)));
    assert_eq!(doc.video_ruler_origin(9), Some(Point::ZERO), "out of range: the first artboard");
}

#[test]
fn video_rulers_show_beside_the_window_rulers_and_not_in_presentation_mode() {
    let mut app = app();
    let ctx = context();
    app.ui.view.rulers = true;
    app.ui.view.video_rulers = true;
    labels(&mut app, &ctx);
    let at = corner(&mut app, 0);
    let texts = labels(&mut app, &ctx);
    assert_eq!(zeros_at(&texts, at), (true, true));
    // The window's top ruler still has its labels along the canvas's top edge.
    let canvas_top = app.canvas_rect.unwrap().top();
    assert!(texts.iter().any(|(_, p, _)| p.y < canvas_top && p.y > canvas_top - 20.0), "{texts:?}");
    app.ui.screen_mode = 3;
    assert_eq!(zeros_at(&labels(&mut app, &ctx), at), (false, false));
}
