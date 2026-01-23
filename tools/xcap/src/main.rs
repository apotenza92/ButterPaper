use clap::Parser;
use xcap::Window;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "xcap")]
struct Args {
    #[arg(short, long)]
    title: Option<String>,

    #[arg(short, long, default_value = "screenshot.png")]
    output: PathBuf,

    #[arg(long)]
    list: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let windows = Window::all()?;

    if args.list {
        for w in &windows {
            println!("{}: {} ({}x{})", w.id()?, w.title()?, w.width()?, w.height()?);
        }
        return Ok(());
    }

    let window = if let Some(title) = &args.title {
        windows.into_iter()
            .find(|w| w.title().map(|t| t.to_lowercase().contains(&title.to_lowercase())).unwrap_or(false))
            .ok_or_else(|| format!("No window found matching '{}'", title))?
    } else {
        windows.into_iter().next().ok_or("No windows found")?
    };

    println!("Capturing: {} ({}x{})", window.title()?, window.width()?, window.height()?);
    let image = window.capture_image()?;
    image.save(&args.output)?;
    println!("Saved to: {:?}", args.output);
    Ok(())
}
