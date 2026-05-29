use anyhow::Result;
use ideot::app::App;

fn main() -> Result<()> {
    let root = std::env::current_dir()?;
    let _app = App::new(root);
    println!("ideot MVP skeleton");
    Ok(())
}
