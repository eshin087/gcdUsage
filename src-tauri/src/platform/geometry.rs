//! Geometry shared by the native strip and tests; no shell customization required.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HoverDecision {
    Switch(usize),
    Show,
    Keep,
    Hide,
}
pub fn hover_decision(
    current: isize,
    over: Option<usize>,
    over_popup: bool,
    visible: bool,
    dragging: bool,
    entered_ms: u128,
    away_ms: u128,
) -> HoverDecision {
    if dragging {
        return HoverDecision::Hide;
    }
    if let Some(index) = over {
        if current != index as isize {
            HoverDecision::Switch(index)
        } else if !visible && entered_ms >= 220 {
            HoverDecision::Show
        } else {
            HoverDecision::Keep
        }
    } else if over_popup || away_ms <= 350 {
        HoverDecision::Keep
    } else {
        HoverDecision::Hide
    }
}
pub fn drag_origin(
    start: (i32, i32),
    current: (i32, i32),
    origin: (i32, i32),
    locked: bool,
    dragging: bool,
) -> Option<(i32, i32)> {
    let (dx, dy) = (current.0 - start.0, current.1 - start.1);
    (!locked && (dragging || dx.abs() + dy.abs() > 4)).then_some((origin.0 + dx, origin.1 + dy))
}
pub fn dock_cell(x: i32, width: i32, scale: f64) -> usize {
    let inset = (7.0 * scale).round() as i32;
    let gap = (5.0 * scale).round() as i32;
    let cell_width = ((width - inset * 2 - gap * 4) / 5).max(1);
    ((x - inset).max(0) / (cell_width + gap).max(1)).min(4) as usize
}

pub fn popup_origin(dock: Rect, work: Rect, size: (i32, i32), gap: i32) -> (i32, i32) {
    let x = (dock.right - size.0).clamp(work.left, (work.right - size.0).max(work.left));
    let above = dock.top - size.1 - gap;
    let y = if above >= work.top {
        above
    } else if dock.bottom + gap + size.1 <= work.bottom {
        dock.bottom + gap
    } else {
        work.top
    };
    (x, y.clamp(work.top, (work.bottom - size.1).max(work.top)))
}

pub fn strip_origin(
    monitor: Rect,
    work: Rect,
    size: (i32, i32),
    saved: Option<(i32, i32)>,
    anchored: bool,
    margin: i32,
) -> (i32, i32) {
    let (width, height) = size;
    let mut origin = saved.unwrap_or((work.right - width - margin, work.bottom - height - margin));
    if anchored {
        // The work area excludes the taskbar. A hidden taskbar can leave no
        // inset; use the normal bottom edge in that case.
        let gaps = [
            monitor.bottom - work.bottom,
            work.top - monitor.top,
            work.left - monitor.left,
            monitor.right - work.right,
        ];
        let edge = gaps
            .iter()
            .enumerate()
            .filter(|(_, gap)| **gap > 0)
            .max_by_key(|(_, gap)| **gap)
            .map(|(edge, _)| edge)
            .unwrap_or(0);
        origin = match edge {
            1 => (work.right - width - margin, work.top),
            2 => (work.left, work.bottom - height - margin),
            3 => (work.right - width, work.bottom - height - margin),
            _ => (work.right - width - margin, work.bottom - height),
        };
    }
    (
        origin
            .0
            .clamp(work.left, (work.right - width).max(work.left)),
        origin
            .1
            .clamp(work.top, (work.bottom - height).max(work.top)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hover_waits_switches_and_stays_open_between_dock_and_card() {
        assert_eq!(
            hover_decision(-1, Some(0), false, false, false, 1000, 0),
            HoverDecision::Switch(0)
        );
        assert_eq!(
            hover_decision(0, Some(0), false, false, false, 219, 0),
            HoverDecision::Keep
        );
        assert_eq!(
            hover_decision(0, Some(0), false, false, false, 220, 0),
            HoverDecision::Show
        );
        assert_eq!(
            hover_decision(0, Some(2), false, true, false, 500, 0),
            HoverDecision::Switch(2)
        );
        assert_eq!(
            hover_decision(2, None, true, true, false, 500, 900),
            HoverDecision::Keep
        );
        assert_eq!(
            hover_decision(2, None, false, true, false, 500, 350),
            HoverDecision::Keep
        );
        assert_eq!(
            hover_decision(2, None, false, true, false, 500, 351),
            HoverDecision::Hide
        );
        assert_eq!(
            hover_decision(2, Some(2), false, true, true, 500, 0),
            HoverDecision::Hide
        );
    }
    #[test]
    fn click_drag_threshold_and_lock_preserve_expected_behavior() {
        assert_eq!(
            drag_origin((10, 10), (11, 11), (100, 200), false, false),
            None
        );
        assert_eq!(
            drag_origin((10, 10), (30, 20), (100, 200), false, false),
            Some((120, 210))
        );
        assert_eq!(
            drag_origin((10, 10), (30, 20), (100, 200), true, true),
            None
        );
        assert_eq!(
            drag_origin((10, 10), (10, 10), (100, 200), false, true),
            Some((100, 200))
        );
    }
    #[test]
    fn scaled_dock_hit_testing_matches_five_card_layout() {
        for s in [0.9, 1.2, 1.6, 2.4] {
            let width = (800.0 * s) as i32;
            for i in 0..5 {
                assert_eq!(
                    dock_cell((width as f64 * (i as f64 + 0.5) / 5.0) as i32, width, s),
                    i
                )
            }
        }
    }
    #[test]
    fn hover_prefers_above_and_clamps_on_small_and_negative_monitors() {
        let work = Rect {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1040,
        };
        let dock = Rect {
            left: -800,
            top: 900,
            right: -20,
            bottom: 1000,
        };
        assert_eq!(popup_origin(dock, work, (600, 580), 6), (-620, 314));
        assert_eq!(
            popup_origin(
                Rect {
                    top: 10,
                    bottom: 100,
                    ..dock
                },
                work,
                (600, 580),
                6
            ),
            (-620, 106)
        );
        assert_eq!(
            popup_origin(
                dock,
                Rect {
                    bottom: 480,
                    ..work
                },
                (2000, 600),
                6
            ),
            (-1920, 0)
        );
    }
    const SCREEN: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };
    #[test]
    fn anchors_to_all_taskbar_edges() {
        let size = (500, 56);
        assert_eq!(
            strip_origin(
                SCREEN,
                Rect {
                    bottom: 1032,
                    ..SCREEN
                },
                size,
                Some((100, 100)),
                true,
                8
            ),
            (1412, 976)
        );
        assert_eq!(
            strip_origin(SCREEN, Rect { top: 48, ..SCREEN }, size, None, true, 8),
            (1412, 48)
        );
        assert_eq!(
            strip_origin(SCREEN, Rect { left: 48, ..SCREEN }, size, None, true, 8),
            (48, 1016)
        );
        assert_eq!(
            strip_origin(
                SCREEN,
                Rect {
                    right: 1872,
                    ..SCREEN
                },
                size,
                None,
                true,
                8
            ),
            (1372, 1016)
        );
    }
    #[test]
    fn negative_monitor_coordinates_and_scaled_strip_stay_in_work_area() {
        let screen = Rect {
            left: -2560,
            top: -200,
            right: 0,
            bottom: 1240,
        };
        let work = Rect {
            bottom: 1168,
            ..screen
        };
        assert_eq!(
            strip_origin(screen, work, (750, 84), None, true, 12),
            (-762, 1084)
        );
        assert_eq!(
            strip_origin(screen, work, (750, 84), Some((4000, -2000)), false, 12),
            (-750, -200)
        );
    }
    #[test]
    fn floating_position_is_preserved_and_hidden_taskbar_has_a_fallback() {
        assert_eq!(
            strip_origin(SCREEN, SCREEN, (500, 56), Some((80, 90)), false, 8),
            (80, 90)
        );
        assert_eq!(
            strip_origin(SCREEN, SCREEN, (500, 56), None, true, 8),
            (1412, 1024)
        );
    }
}
