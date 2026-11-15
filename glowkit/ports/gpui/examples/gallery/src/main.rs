use gpui::*;
struct Hello;
impl Render for Hello {
    fn render(&mut self, w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        eprintln!("RENDER"); w.request_animation_frame();
        div().size_full().bg(rgb(0xff3344)).text_color(rgb(0xfafafa)).text_size(px(40.)).child("Hello GPUI")
    }
}
fn main() {
    env_logger::init();
    Application::new().run(|cx: &mut App| {
        let b = Bounds::centered(None, size(px(800.), px(600.)), cx);
        cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(b)), ..Default::default() }, |_, cx| cx.new(|_| Hello)).unwrap();
    });
}
