use skindraw::app::SkinDrawApp;

fn main() -> eframe::Result {
    eframe::run_native(
        "SkinDraw",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([960.0, 720.0])
                .with_min_inner_size([640.0, 480.0]),
            ..Default::default()
        },
        Box::new(|creation| Ok(Box::new(SkinDrawApp::new(creation)))),
    )
}
