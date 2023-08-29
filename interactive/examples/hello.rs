use ibis_interactive::Window;

fn main() {
    let window = Window {
        title: "hello".to_string(),
        width_px: 960,
        height_px: 720,
    };
    window.run().unwrap();
}
