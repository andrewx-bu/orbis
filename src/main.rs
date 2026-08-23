mod app;
pub mod renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
