//! Geometry shared by the native strip and tests; no shell customization required.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
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
