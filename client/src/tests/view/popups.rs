use super::keep_on_screen;
use iced::{Point, Size};

#[test]
fn menus_near_the_bottom_or_right_move_back_onto_the_window() {
    let window = Size::new(1000.0, 700.0);
    // Opened near the bottom right: moved so the whole menu is visible.
    let at = keep_on_screen(Point::new(990.0, 690.0), Size::new(200.0, 300.0), window);
    assert_eq!(at, Point::new(792.0, 392.0));
    // Room to spare: stays where it was opened.
    assert_eq!(
        keep_on_screen(Point::new(100.0, 50.0), Size::new(200.0, 300.0), window),
        Point::new(100.0, 50.0)
    );
    // A menu taller than the window stays at the top margin rather than going off the top.
    assert_eq!(
        keep_on_screen(Point::new(10.0, 10.0), Size::new(200.0, 900.0), window).y,
        8.0
    );
}
